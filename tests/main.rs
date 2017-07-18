extern crate buf_file;

use std::io::{self, Read, Write, Seek, SeekFrom, Cursor};

use buf_file::BufFile;

#[test]
fn simple_test() {
    let mut data = Cursor::new(vec![1u8; 1024 * 1024 * 32]);
    {
        let mut file = BufFile::with_capacity(4, 4 * 1024, &mut data).unwrap();
        file.write_all(&[0, 1, 2]).unwrap();
        file.seek(SeekFrom::Current(-3)).unwrap();
        let mut buf = [0; 3];
        file.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], &[0, 1, 2]);
        file.seek(SeekFrom::Start(5)).unwrap();
        file.write_all(&[5]).unwrap();
        for i in 1..32 {
            file.seek(SeekFrom::Start(1024 * 1024 * i)).unwrap();
            file.write_all(&[0, 0]).unwrap();
        }
        for i in 1..32 {
            file.seek(SeekFrom::Start(1024 * 1024 * i)).unwrap();
            file.read_exact(&mut buf).unwrap();
            assert_eq!(buf, [0, 0, 1])
        }
    }
    let data = data.into_inner();
    assert_eq!(data.len(), 1024 * 1024 * 32);
    assert_eq!(&data[..7], &[0, 1, 2, 1, 1, 5, 1]);
    for i in 1..32 {
        let offset = 1024 * 1024 * i;
        assert_eq!(&data[offset..offset + 3], &[0, 0, 1]);
    }
}

#[test]
fn write_counts() {
    struct Writer {
        inner: Cursor<Vec<u8>>,
        write_count: u32,
        read_count: u32,
    }
    impl Write for Writer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_count += 1;
            self.inner.write(buf)
        }
        
        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }
    impl Read for Writer {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.read_count += 1;
            self.inner.read(buf)
        }
    }
    impl Seek for Writer {
        fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
            self.inner.seek(from)
        }
    }
    let mut data = Writer {
        inner: Cursor::new(vec![0; 1024]),
        write_count: 0,
        read_count: 0,
    };
    {
        let mut file = BufFile::with_capacity(4, 16, &mut data).unwrap();
        file.write_all(&[8; 32]).unwrap();
        for i in (0..16).rev() {
            // Skip every other slab
            file.seek(SeekFrom::Start(i * 32)).unwrap();
            if i % 2 == 0 {
                let mut buf = [0; 8];
                file.read_exact(&mut buf).unwrap();
                if i == 0 {
                    assert_eq!(buf, [8; 8]);
                } else {
                    assert_eq!(buf, [0; 8]);
                }
            } else {
                file.write_all(&[1, 2, 3, 4]).unwrap();
            }
        }
    }
    // Extra 2 from writting 2 slabs early, then writing to all in rev order
    assert_eq!(data.read_count, 18);
    assert_eq!(data.write_count, 10);
    let data = data.inner.into_inner();
    assert_eq!(data.len(), 1024);
}