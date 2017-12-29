#![feature(libc)]
extern crate libc;

use self::libc::{c_void, c_char};
use std::ffi::{CString, CStr};
use std::ptr;
use std::error::Error;
use std::mem;
use std::sync::Mutex;

/// Intended for future use
pub enum IvyApplicationEvent {
    IvyApplicationConnected,
    IvyApplicationDisconnected,
    IvyApplicationCongestion,
    IvyApplicationDecongestion,
    IvyApplicationFifoFull,
}

/// Minimal struct representation compatible with Ivy library
#[repr(C, packed)]
pub struct IvyClientPtr {
    next: *mut IvyClientPtr,
}

/// Minimal struct representation compatible with Ivy library
#[derive(Clone)]
#[repr(C, packed)]
pub struct MsgRcvPtr {
    next: *mut MsgRcvPtr,
}

/// Generic struct holding callback data
pub struct IvyMessage {
    regexpr: String,
    data: Mutex<Vec<Vec<String>>>,
    msg_ptr: Option<MsgRcvPtr>,
}

#[allow(dead_code)]
impl IvyMessage {
	pub fn new() -> IvyMessage {
		IvyMessage {
			regexpr: String::new(),
			data: Mutex::new(vec![]),
			msg_ptr: None,
		}
	}

    /// Simple callback
    #[allow(dead_code)]
    pub fn callback(&mut self, data: Vec<String>) {
        // append the vector with new data
        let mut lock = self.data.lock();
	    if let Ok(ref mut mutex) = lock {
			mutex.push(data);
	    } else {
	        println!("Ivy callback: mutex lock failed");
	    }
    }

    /// Unbind message
    pub fn ivy_unbind_msg(&mut self) -> bool {
    	let flag;
    	match self.msg_ptr {
    		Some(ref ptr) => {
	    			let p = ptr.clone();
    			    unsafe {
				        IvyUnbindMsg(p);
				    }
    			    flag = true;
    		}
    		None => {
    			flag = false;
    		}
    	};
    	if flag {
    		self.msg_ptr = None;
    	}
    	flag
    }

	/// Change regular expression of an existing message
	pub fn ivy_change_msg(&mut self, regexpr: String) -> bool {
		self.regexpr = regexpr.clone();
	    let regexpr = CString::new(regexpr).unwrap();
	    
	    let flag;
	    let msg_ptr;
	    match self.msg_ptr {
	    	Some(ref mut ptr) => {
	    		let p = ptr.clone();
		    	unsafe {
		    		msg_ptr = Some(IvyChangeMsg(p,regexpr.as_ptr()));
		    	};
		    	flag = true;
	    	}
	    	None => {
	    		msg_ptr = None;
	    		flag = false;
	    	},
	    };
	    if flag {
    		self.msg_ptr = msg_ptr;
    	}
    	flag
	}


	/// Bind ivy message to a simple callback with given regexpr
    pub fn ivy_bind_msg<F>(&mut self, cb: F, regexpr: String) -> bool
    where
        F: Fn(&mut IvyMessage, Vec<String>),
    {
    	self.regexpr = regexpr.clone();
        let regexpr = CString::new(regexpr).unwrap();
        
        let msg_ptr;
        {
	        let boxed_cb: Box<(Box<Fn(&mut IvyMessage, Vec<String>)>, &mut IvyMessage)> =
	            Box::new((Box::new(cb), self));
	        msg_ptr = unsafe {
			            Some(IvyBindMsg(
			                apply_closure,
			                Box::into_raw(boxed_cb) as *const c_void,
			                regexpr.as_ptr(),
			            ))
			        };
        }
        self.msg_ptr = msg_ptr;
        match self.msg_ptr {
        	Some(_) => true,
        	None => false,
        }
    }
}

#[allow(dead_code)]
extern "C" fn apply_closure(_app: IvyClientPtr,
                            user_data: *mut c_void,
                            argc: i32,
                            argv: *const *const c_char) {
    // parse argv into String vector for easier management
    let mut v: Vec<String> = vec![];
    for i in 0..argc as isize {
        unsafe {
            let ptr = argv.offset(i);
            v.push(String::from(CStr::from_ptr(*ptr).to_str().unwrap()));
        }
    }
    
    let payload: &mut (Box<Fn(&mut IvyMessage, Vec<String>) -> ()>, &mut IvyMessage) =
        unsafe { mem::transmute(user_data) };
    
    payload.0(&mut payload.1, v);
}

#[allow(dead_code)]
#[link(name = "ivy")]
extern "C" {
    fn IvyInit(
        app_name: *const c_char,
        ready_msg: *const c_char,
        callback: *const c_void,
        app_data: *const c_void,
        die_callback: *const c_void,
        die_data: *const c_void,
    );
    fn IvyStart(bus_addr: *const c_char);
    fn IvyStop();
    fn IvyMainLoop();
    fn IvySendMsg(fmt_message: *const c_char, ...);
    pub fn IvyBindMsg(
        callback: extern "C" fn(app: IvyClientPtr,
                                user_data: *mut c_void,
                                argc: i32,
                                argv: *const *const c_char),
        user_data: *const c_void,
        regexpr: *const c_char,
        ...
    ) -> MsgRcvPtr;
    fn IvyUnbindMsg(id: MsgRcvPtr);
    fn IvyChangeMsg(msg: MsgRcvPtr, fmt_regex: *const c_char, ...) -> MsgRcvPtr;
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
        IvyInit(
            app_name.as_ptr(),
            ready_msg.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
    }
}

/// Start Ivy bus
///
/// Specify the bus addres as `X.X.X.X:YYY`,
/// the default is `127.255.255.255`, port `2010`
///
pub fn ivy_start(bus_addr: Option<String>) {
    match bus_addr {
        Some(addr) => unsafe {
            IvyStart(CString::new(addr).unwrap().as_ptr());
        },
        None => unsafe {
            IvyStart(CString::new("").unwrap().as_ptr());
        },
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


#[cfg(test)]
mod tests {
    extern crate libc;

    use super::*;

    use std::error::Error;
    use std::{thread, time};

    // reference AC_ID
    const AC_ID: i32 = 69;

    fn thread2() -> Result<(), Box<Error>> {
        let wind_msg = format!("{} NPS_WIND {} {} {}", AC_ID, 1.0, 2.0, 3.0);
        ivy_send_msg(wind_msg);
        thread::sleep(time::Duration::from_millis(10000));
        Ok(())
    }

    #[test]
    fn main_test() {
        ivy_init(String::from("RUST_IVY"), String::from("RUST_IVY Ready"));
        ivy_start(None);
        
        let mut cb1 = IvyMessage::new();
        let mut cb2 = IvyMessage::new();

        assert_eq!(true,cb1.ivy_bind_msg(IvyMessage::callback, String::from("^(\\S*) DL_SETTING (\\S*)")));
        assert_eq!(true,cb2.ivy_bind_msg(IvyMessage::callback, String::from("(.*)")));

        let _t1 = thread::spawn(move || if let Err(e) = ivy_main_loop() {
            println!("Error in Ivy main loop: {}", e);
        } else {
            println!("ivy main loop finished finished");
        });

        let t2 = thread::spawn(move || if let Err(e) = thread2() {
            println!("Error in thread2: {}", e);
        } else {
            println!("thread2 finished finished");
        });

		// this will unbind the callback from the message
        assert_eq!(true,cb2.ivy_unbind_msg());
        
        // this will allow us to change the already bound message
        assert_eq!(true,cb1.ivy_change_msg(String::from("^(\\S*) DL_SETTING (\\S*) (\\S*) (\\S*)")));

        t2.join().expect("Error waiting for t2 to finish");

        ivy_stop();
    }
}
