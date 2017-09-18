#![feature(test)]

extern crate buf_file;
extern crate test;
extern crate tempfile;

use buf_file::BufFile;
use test::Bencher;
use std::io::{ Seek, Write, Read, SeekFrom, BufWriter, BufReader };
use tempfile::tempfile;

#[bench]
fn write_16_mb_buf_file(b: &mut Bencher) {
    b.iter(|| {
        let mut test_buffile = BufFile::new(tempfile().unwrap()).unwrap();
        let kb = vec![0u8; 1024];
        for _ in 0..1024*16 {
            test_buffile.write(&kb).unwrap();
        }
    });
    b.bytes = 1024 * 16 * 1024;
}

#[bench]
fn read_16_mb_buf_file(b: &mut Bencher) {
    let f = tempfile::NamedTempFile::new().unwrap();
    let mut test_buffile = BufFile::new(f.reopen().unwrap()).unwrap();
    let kb = vec![0u8; 1024*16];
    test_buffile.write(&kb).unwrap();
    drop(test_buffile);
    b.iter(|| {
        let mut test_buffile = BufFile::new(f.reopen().unwrap()).unwrap();
        let mut kb = vec![0u8; 1024*1024*16];
        for i in 0..1024*16 {
            test_buffile.read(&mut kb[i * 1024 ..(i + 1) * 1024]).unwrap();
        }
    });
    b.bytes = 1024 * 16 * 1024;
}

#[bench]
fn read_16_mb_bufreader(b: &mut Bencher) {
    let f = tempfile::NamedTempFile::new().unwrap();
    let mut test_buffile = BufFile::new(f.reopen().unwrap()).unwrap();
    let kb = vec![0u8; 1024*16];
    test_buffile.write(&kb).unwrap();
    drop(test_buffile);
    b.iter(|| {
        let mut test_buffile = BufReader::new(f.reopen().unwrap());
        let mut kb = vec![0u8; 1024*1024*16];
        for i in 0..1024*16 {
            test_buffile.read(&mut kb[i * 1024 ..(i + 1) * 1024]).unwrap();
        }
    });
    b.bytes = 1024 * 16 * 1024;
}

#[bench]
fn write_16_mb_buf_write(b: &mut Bencher) {
    b.iter(|| {
        let mut test_buffile = BufWriter::new(tempfile().unwrap());
        let kb = vec![0u8; 1024];
        for _ in 0..1024*16 {
            test_buffile.write(&kb).unwrap();
        }
    });
    b.bytes = 1024 * 16 * 1024;
}

#[bench]
fn write_and_read_16_mb_file_buf(b: &mut Bencher) {
    b.iter(|| {
        let mut test_buffile = BufFile::new(tempfile().unwrap()).unwrap();
        let kb = vec![0u8; 1024];
        for _ in 0..1024*16 {
            test_buffile.write(&kb).unwrap();
        }
        test_buffile.seek(SeekFrom::Start(0)).unwrap();
        let mut big_buffer = Vec::<u8>::with_capacity(1024*1024*16);
        for i in 0..1024*16 {
            big_buffer.extend(kb.iter().cloned());
            test_buffile.read(&mut big_buffer[i * 1024 ..]).unwrap();
        }
    });
    b.bytes = 1024 * 16 * 1024;
}

#[bench]
fn write_and_read_16_mb_bufwrite_bufread(b: &mut Bencher) {
    b.iter(|| {
        let mut test_buffile = BufWriter::new(tempfile().unwrap());
        let kb = vec![0u8; 1024];
        for _ in 0..1024*16 {
            test_buffile.write(&kb).unwrap();
        }
        let mut file = test_buffile.into_inner().unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut test_bufread = BufReader::new(file);
        let mut big_buffer = Vec::<u8>::with_capacity(1024*1024*16);
        for i in 0..1024*16 {
            big_buffer.extend(kb.iter().cloned());
            test_bufread.read(&mut big_buffer[i * 1024 ..]).unwrap();
        }
    });
    b.bytes = 1024 * 16 * 1024;
}

