extern crate buf_file;

use std::io::{Read, Write, Seek, SeekFrom, Cursor};

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