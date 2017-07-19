#![feature(rand)]
#![feature(test)]
extern crate rand;
extern crate test;

mod file_buffer;
pub use file_buffer::BufFile;


#[cfg(test)]
mod bench {
    use test::Bencher;
    use file_buffer::*;
    use std::fs::*;
    use std::io::{ Seek, Write, Read, SeekFrom, BufWriter, BufReader };

    #[bench]
    fn write_16_mb_buf_file(b: &mut Bencher) {
        b.iter(|| {
            let mut test_buffile = {
                BufFile::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open("write_16_mb_buf_file")
                        .unwrap()
                    ).unwrap()
            };
            let kb = vec![0u8; 1024];
            for _ in 0..1024*16 {
                test_buffile.write(&kb).unwrap();
            }
        });
    }

    #[bench]
    fn read_16_mb_buf_file(b: &mut Bencher) {
        let mut test_buffile = {
            BufFile::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open("read_16_mb_buf_file")
                    .unwrap()
                ).unwrap()
        };
        let kb = vec![0u8; 1024*16];
        test_buffile.write(&kb).unwrap();
        {
            let _ = test_buffile;
        };
        b.iter(|| {
            let mut test_buffile = {
                BufFile::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(false)
                        .create(true)
                        .open("read_16_mb_buf_file")
                        .unwrap()
                    ).unwrap()
            };
            let mut kb = vec![0u8; 1024*1024*16];
            for i in 0..1024*16 {
                test_buffile.read(&mut kb[i * 1024 ..(i + 1) * 1024]).unwrap();
            }
        });
    }

    #[bench]
    fn read_16_mb_bufreader(b: &mut Bencher) {
        let mut test_buffile = {
            BufFile::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open("read_16_mb_bufreader")
                    .unwrap()
                ).unwrap()
        };
        let kb = vec![0u8; 1024*16];
        test_buffile.write(&kb).unwrap();
        {
            let _ = test_buffile;
        };
        b.iter(|| {
            let mut test_buffile = {
                BufReader::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(false)
                        .create(true)
                        .open("read_16_mb_bufreader")
                        .unwrap()
                    )
            };
            let mut kb = vec![0u8; 1024*1024*16];
            for i in 0..1024*16 {
                test_buffile.read(&mut kb[i * 1024 ..(i + 1) * 1024]).unwrap();
            }
        });
    }

    #[bench]
    fn write_16_mb_buf_write(b: &mut Bencher) {
        b.iter(|| {
            use std::io::BufWriter;
            let mut test_buffile = {
                BufWriter::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open("write_16_mb_buf_file")
                        .unwrap()
                    )
            };
            let kb = vec![0u8; 1024];
            for _ in 0..1024*16 {
                test_buffile.write(&kb).unwrap();
            }
        });
    }

    #[bench]
    fn write_and_read_16_mb_file_buf(b: &mut Bencher) {
        b.iter(|| {
            let mut test_buffile = {
                BufFile::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open("write_16_mb_file_buf")
                        .unwrap()
                    ).unwrap()
            };
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
    }

    #[bench]
    fn write_and_read_16_mb_bufwrite_bufread(b: &mut Bencher) {
        b.iter(|| {
            let mut test_buffile = {
                BufWriter::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open("write_16_mb_file_buf")
                        .unwrap()
                    )
            };
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
    }
}

#[cfg(test)]
mod tests {
    // This test verifies that the BufFile behaves exactly like a file when reading, writing, and seeking.
    // It randomly seeks and writes data, and verifies everything is completely equal with the actual file.
    #[test]
    fn test_file_buffer() {
        use std::fs::OpenOptions;
        use std::io::{ Seek, SeekFrom, Read, Write };
        use file_buffer::*;
        use std::time::{ SystemTime };
        use rand::Rng;
        use rand::*;

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
}
