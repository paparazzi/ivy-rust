#![feature(libc)]
extern crate libc;
extern crate ivyrust;

use std::error::Error;
use std::{thread, time};

const AC_ID: i32 = 69;


fn test(n: Vec<String>) {
	println!("TEST!");
	println!("Data={:?}",n);
}


fn thread2() -> Result<(), Box<Error>> {
	loop {
		let wind_msg = format!("{} NPS_WIND {} {} {}", AC_ID, 1.0, 2.0, 3.0);
		ivyrust::ivy_send_msg(wind_msg);
		thread::sleep(time::Duration::from_millis(10000));	
	}
}

fn main() {
	ivyrust::ivy_init(String::from("RUST_IVY"),String::from("RUST_IVY Ready"));
	ivyrust::ivy_start(None);
	let p1 = ivyrust::ivy_bind_msg(test, String::from("^(\\S*) DL_SETTING (\\S*) (\\S*) (\\S*)"));
	
	let ptr = ivyrust::ivy_bind_msg(test, String::from("(.*)"));
	//ivyrust::ivy_unbind_msg(&ptr);
	
	let t1 = thread::spawn(move || {
		if let Err(e) =  ivyrust::ivy_main_loop() {
			println!("Error in Ivy main loop: {}", e);
		} else {
			println!("ivy main loop finished finished");	
		}
	});
	
	
	let t2 = thread::spawn(move || {
		if let Err(e) =  thread2() {
			println!("Error in thread2: {}", e);
		} else {
			println!("thread2 finished finished");	
		}
	});
	 
	 

    println!("It works");
    
    ivyrust::ivy_unbind_msg(ptr);
    ivyrust::ivy_change_msg(p1, String::from("^(\\S*) DL_SETTING (\\S*)"));
    
    t1.join().expect("Error waiting for t1 to finish");
    t2.join().expect("Error waiting for t2 to finish");
    
    ivyrust::ivy_stop();
}