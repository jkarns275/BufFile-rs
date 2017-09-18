extern crate rand;
extern crate buf_file;

use std::io::prelude::*;
use std::io::SeekFrom;
use std::fs::OpenOptions;
use std::time::SystemTime;

use rand::{Rng, SeedableRng};
use rand::XorShiftRng;

use buf_file::BufFile;

#[test]
#[should_panic]
fn test_seek_start_error() {
    let mut test_file = BufFile::new(OpenOptions::new().read(true).write(true).truncate(true).create(true).open("test112324").unwrap()).unwrap();
    test_file.seek(SeekFrom::Start(1)).unwrap();
}

#[test]
#[should_panic]
fn test_seek_end_error() {
    let mut test_file = BufFile::new(OpenOptions::new().read(true).write(true).truncate(true).create(true).open("test112324").unwrap()).unwrap();
    test_file.seek(SeekFrom::End(1)).unwrap();
}

#[test]
#[should_panic]
fn test_seek_current_error() {
    let mut test_file = BufFile::new(OpenOptions::new().read(true).write(true).truncate(true).create(true).open("test112324").unwrap()).unwrap();
    test_file.seek(SeekFrom::Current(1)).unwrap();
}

// This test verifies that the BufFile behaves exactly like a file when reading, writing, and seeking.
// It randomly seeks and writes data, and verifies everything is completely equal with the actual file.
#[test]
fn test_file_buffer() {

    let now = SystemTime::now();

    let mut test_file = OpenOptions::new().read(true).write(true).truncate(true).create(true).open("yzyy").unwrap();
    let t = OpenOptions::new().read(true).write(true).truncate(true).create(true).open("zyys").unwrap();
    let mut test_buffile = BufFile::new(t).unwrap();

    let mut rng = XorShiftRng::from_seed([0, 1, 377, 6712]);
    test_file.write(&[0]).unwrap();
    test_buffile.write(&[0]).unwrap();

    for _ in 0..100 {
        for _ in 0..1000 {
            let x = rng.gen::<u64>();
            let a = test_file.seek(SeekFrom::End(0)).unwrap();
            let b = test_buffile.seek(SeekFrom::End(0)).unwrap();
            if a != b {
                panic!("len_check fail: {} != buf: {}", a, b);
            }
            let c = test_file.seek(SeekFrom::Start(x % (a as u64))).unwrap();
            let d = test_buffile.seek(SeekFrom::Start(x % (a as u64))).unwrap();
            if c != d {
                panic!(" c: {} != d: {}", c, d);
            }
            let y = rng.gen::<u32>() % 100;
            for _ in 0..y {
                let z = rng.gen::<u8>();
                test_file.write(&[z]).unwrap();
                test_buffile.write(&[z]).unwrap();
            }
        }
        let a = test_file.seek(SeekFrom::End(0)).unwrap();
        let b = test_buffile.seek(SeekFrom::End(0)).unwrap();
        if a != b {
            panic!("len_check 2 fail: {} != buf: {}", a, b);
        }
        let _ = test_file.seek(SeekFrom::Start(0)).unwrap();
        let _ = test_buffile.seek(SeekFrom::Start(0)).unwrap();
        for _ in 0..a {
            let mut b1 = [0u8];
            let mut b2 = [0u8];
            test_file.read(&mut b1).unwrap();
            test_buffile.read(&mut b2).unwrap();
            if b1 != b2 {
                panic!("Data confirmation failed: {:?} != {:?}", b1, b2);
            }
        }
    }

    match now.elapsed() {
        Ok(a) => println!("time for test_file_buffer: {:?}", a),
        Err(_) => panic!("Error measuring time.."),
    };
}
