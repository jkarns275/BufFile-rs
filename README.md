# BufFile-rs
A Buffered File for rust that allows both reading and writing.

```rust
use std::time::{ SystemTime };
use std::io::{ Error, Seek, SeekFrom, Write, Read, BufWriter};
use std::fs::{ File, OpenOptions };
use buf_file::*;

let num_mb = 128;

let dat = vec![1u8; 1024];

// Write some data using a BufWriter, time it
let mut t2 = OpenOptions::new().read(true).write(true).truncate(true).create(true).open("xasd.tree").unwrap();
let mut file2 = BufWriter::new(t2);

let now = SystemTime::now();

for i in 0..num_mb * 1024 {
    file2.write(&dat[0..]).unwrap();
}

match now.elapsed() {
    Ok(a) => {
        let seconds = a.as_secs() as f64 + (a.subsec_nanos() as f64 / 1e9f64);
        println!("time to write {} megabytes with BufWriter: {:?}\n{} mb / s WRITE", num_mb, seconds, num_mb as f64 / seconds);
    },
    Err(_) => panic!("Error measuring time.."),
};

// Write some data using a BufFile, time it
let mut t1 = OpenOptions::new().read(true).write(true).truncate(true).create(true).open("test.tree").unwrap();
let mut file = BufFile::with_capacity(8, t1).unwrap();

let now = SystemTime::now();

for i in 0..num_mb * 1024 {
    file.write(&dat[0..]).unwrap();
}

match now.elapsed() {
    Ok(a) => {
        let seconds = a.as_secs() as f64 + (a.subsec_nanos() as f64 / 1e9f64);
        println!("time to write {} megabytes with BufFile: {:?}\n{} mb / s WRITE", num_mb, seconds, num_mb as f64 / seconds);
    },
    Err(_) => panic!("Error measuring time.."),
};
// Sample output (from output on windows x64, with an SSD):
//
// time to write 128 megabytes with BufWriter: 0.1323519
// 967.1187191117015 mb / s WRITE
// time to write 128 megabytes with BufFile: 0.1970247
// 649.6647374669268 mb / s WRITE
```

