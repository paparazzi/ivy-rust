#![feature(libc)]
extern crate libc;

use libc::{c_void,c_char};
use std::ffi::{CString,CStr};
use std::ptr;
use std::error::Error;
use std::mem;


/// Minimal struct representation compatible with Ivy library
#[repr(C,packed)]
pub struct IvyClientPtr {
	next: *mut IvyClientPtr,
}

/// Minimal struct representation compatible with Ivy library
#[repr(C,packed)]
pub struct MsgRcvPtr {
	next: *mut MsgRcvPtr,
}

/// Intended for future use
pub enum IvyApplicationEvent { 
	IvyApplicationConnected,
	IvyApplicationDisconnected,
	IvyApplicationCongestion,
	IvyApplicationDecongestion,
	IvyApplicationFifoFull,
}

#[link(name = "ivy")]
extern {
	fn IvyInit(app_name: *const c_char, ready_msg: *const c_char, callback: *const c_void, app_data: *const c_void, die_callback: *const c_void, die_data: *const c_void);
	fn IvyStart(bus_addr: *const c_char);
	fn IvyStop();
    fn IvyMainLoop();    
	fn IvySendMsg(fmt_message: *const c_char, ...);
	fn IvyBindMsg(callback: extern fn(app: IvyClientPtr, user_data: *mut c_void, argc: i32, argv: *const *const c_char),
						user_data: *const c_void, regexpr: *const c_char, ...) -> MsgRcvPtr;
	fn IvyUnbindMsg(id: MsgRcvPtr);
	fn IvyChangeMsg (msg: MsgRcvPtr, fmt_regex: *const c_char, ... ) -> MsgRcvPtr;
}


/// Initialize Ivy bus
/// 
/// `app_name` is the name of the application as it will
/// show up on the Ivy bus, `ready_msg` is printed on the bus
/// once your application is ready to listen,
/// 
pub fn ivy_init(app_name: String, ready_msg: String) {
	let app_name = CString::new(app_name).unwrap();
	let ready_msg = CString::new(ready_msg).unwrap();
	unsafe {
		IvyInit(app_name.as_ptr(), ready_msg.as_ptr(), ptr::null(), ptr::null(), ptr::null(), ptr::null());
	}
}


/// Start Ivy bus
///
/// Specify the bus addres as `X.X.X.X:YYY`,
/// the default is `127.255.255.255`, port `2010`
///
pub fn ivy_start(bus_addr: Option<String>) {
	match bus_addr {
		Some(addr) => {
			unsafe {
				IvyStart(CString::new(addr).unwrap().as_ptr());
			}
		},
		None => {
			unsafe {
				IvyStart(CString::new("").unwrap().as_ptr());
			}
		}
	}
}


/// Stop Ivy bus
pub fn ivy_stop() {
	unsafe { 
		IvyStop();
	}
}


/// Run the main loop, typically in a thread
pub fn ivy_main_loop() -> Result<(), Box<Error>> {
	unsafe {
		IvyMainLoop();
	}
	
	Ok(())
}



/// Send a message over Ivy bus
///
/// Note that non-ascii messages are not handled, and might cause `panic!`
pub fn ivy_send_msg(msg: String) {
	let msg = CString::new(msg).unwrap();
	unsafe {
		IvySendMsg(msg.as_ptr());
	}
}



/// Bind a callback for a particular message
///
/// `regexpr` defines which message should be bound, for 
/// example "\^(\\S*) DL_SETTING (\\S*) (\\S*) (\\S*)" will
/// and `cb` is a function callback in form of `Fn(Vec<String>)`
/// 
pub fn ivy_bind_msg<F>(cb: F, regexpr: String) -> MsgRcvPtr
    where F: Fn(Vec<String>)
{
	let regexpr = CString::new(regexpr).unwrap();
	
    // For the reasons articulated above, we create a pointer to a
    // (fat) pointer to the closure
    let boxed_cb: Box<Box<Fn(Vec<String>)>> = Box::new(Box::new(cb));
    // and then we call the C function with our 'apply_closure'
    // function as well as the actual closure:
    unsafe {
        IvyBindMsg(apply_closure, Box::into_raw(boxed_cb) as *const c_void, regexpr.as_ptr())
    }
}  


/// Unbind an existing message
pub fn ivy_unbind_msg(id: MsgRcvPtr) {
	unsafe {
		IvyUnbindMsg(id);
	}
}


/// Change regular expression of an existing message
pub fn ivy_change_msg(id: MsgRcvPtr, regexpr: String) -> MsgRcvPtr {
	let regexpr = CString::new(regexpr).unwrap();
	unsafe {
        IvyChangeMsg(id, regexpr.as_ptr())
    }
}


// This function is what we're actually going to provide as the
// function pointer to call_callback: it takes a number and a void
// pointer...
extern "C" fn apply_closure(_app: IvyClientPtr, user_data: *mut c_void, argc: i32, argv: *const *const c_char) {
	// parse argv into String vector for easier management
	let mut v: Vec<String> = vec![];
    for i in 0..argc as isize {
			unsafe {
				let ptr = argv.offset(i);
				v.push(String::from(CStr::from_ptr(*ptr).to_str().unwrap()));
			}	
		}
    // and here it interprets the void pointer as a reference to a
    // pointer to a function. The two layers are doing something
    // important: pointers to traits in Rust are "fat pointers",
    // i.e. they are actually a function-pointer/data-pointer pair. So
    // what we're doing is getting a pointer to those two pointer
    // pairs, and then using those to call the code with the
    // appropriate data.
    let closure: &mut Box<Fn(Vec<String>) -> ()> = unsafe { mem::transmute(user_data) };
    // And then we can just call the closure!
    closure(v);
}