use std::io::{ Error, ErrorKind, Seek, SeekFrom, Write, Read };
use std::collections::HashMap;

const DEFAULT_SLAB_SIZE: usize = 16 * 1024; // 16KiB
const DEFAULT_NUM_SLABS: usize = 16;

/// A struct representing a section of a file
pub struct Slab {
    /// The data
    pub dat: Vec<u8>,
    /// First byte in the file that is contained in this slab
    start: u64,
    /// Number of times this slab has been accessed.
    uses: u32,
    dirty: bool,
}

impl Slab {
    /// Creates a new slab, drawing it's data from the given file at the given location
    /// Location should be at the beginning of a slab (e.g. a muitiple of SLAB_SIZE)
    pub fn new<R: Read + Seek + ?Sized>(loc: u64, size: usize, reader: &mut R) -> Result<Slab, Error> {
        reader.seek(SeekFrom::Start(loc))?;
        let mut dat = vec![0u8; size];
        // It isn't safe to use read_exact, since the file may end early, so we loop until
        // the slice is full, or until a length of zero is returned (EOF)
        {
            let mut dat = &mut dat[..];
            while !dat.is_empty() {
                match reader.read(dat) {
                    Ok(0) => break,
                    Ok(n) => { let tmp = dat; dat = &mut tmp[n..]; }
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(Slab {
            dat: dat,
            start: loc,
            uses: 0,
            dirty: false,
        })
    }

    /// Write the slab to disk
    pub fn write<W: Write + Seek + ?Sized>(&mut self, writer: &mut W) -> Result<(), Error> {
        if ! self.dirty {
            return Ok(())
        }
        writer.seek(SeekFrom::Start(self.start))?;
        writer.write_all(&self.dat[0..])?;
        self.dirty = false;
        Ok(())
    }
}

pub struct BufFile<F: Read + Write + Seek> {
    /// The maximum number of slabs this BufFile can have
    max_slabs: usize,
    /// Size of a slab
    slab_size: usize,
    /// Used to quickly map a file index to a slab
    map: HashMap<u64, Slab>,
    /// The file to be written to and read from
    file: F,
    /// Represents the current location of the cursor.
    /// This does not reflect the actual location of the cursor in the file.
    pub cursor: u64,
    /// The file index that is the end of the file.
    pub end: u64
}

impl<F: Read + Write + Seek> BufFile<F> {
    #[inline]
    fn slab_mask(&self) -> u64 {
        self.slab_size as u64 - 1
    }
    /// Creates a new BufFile.
    pub fn new(file: F) -> Result<BufFile<F>, Error> {
        Self::with_capacity(DEFAULT_NUM_SLABS, DEFAULT_SLAB_SIZE, file)
    }

    /// Creates a new BufFile with the specified number of slabs.
    pub fn with_capacity(slab_count: usize, slab_size: usize, mut file: F) -> Result<BufFile<F>, Error> {
        assert!(slab_size.is_power_of_two());
        // Find the end of the file, in case the file isnt empty.
        let end = file.seek(SeekFrom::End(0))?;

        // Move the cursor back to the start of the file.
        file.seek(SeekFrom::Start(0))?;
        Ok(BufFile {
            max_slabs: slab_count,
            slab_size: slab_size,
            map: HashMap::new(),
            file,
            cursor: 0,  // Since the cursor is at the start of the file
            end
        })
    }

    /// Adds a slab to the BufFile, if it isn't already present. It will write
    /// the least frequently used slab to disk and load the new one into self.dat,
    /// then return Ok(index), index being an index for self.dat.
    fn fetch_slab(&mut self, loc: u64) -> Result<&mut Slab, Error> {
        let start = loc & !self.slab_mask();

        if self.map.contains_key(&start) {
            return Ok(self.map.get_mut(&start).unwrap());
        }
        // Add up to 2048 bytes if the file is not long enough for this incoming location
        let len = self.end as usize;
        // The end if the file is not as long as it needs to be, write some dummy data (0's) to extend it
        // This behavior will allow some strange behavior through, but it shouldnt't really be harmful
        if len < start as usize + self.slab_size && len < loc as usize {
            let i = vec![0; self.slab_size];
            let dif = len & self.slab_mask() as usize;
            self.file.write_all(&i[0..self.slab_size - dif])?;
            self.end = loc + 1;
        }
        if self.map.len() >= self.max_slabs {
            let mut min_start = 0;
            let mut min_uses = u32::max_value();
            for (&start, slab) in &self.map {
                if slab.uses == 1 {
                    // The minimum number of reads is 1, so if we encounter 1 just break.
                    min_start = slab.start;
                    break;
                }
                if slab.uses < min_uses {
                    min_start = start;
                    min_uses = slab.uses;
                }
            }
            // Unwrap is safe because we find the start above
            let mut old_slab = self.map.remove(&min_start).unwrap();
            old_slab.write(&mut self.file)?;
            // Move the cursor back to where it was
            self.file.seek(SeekFrom::Start(self.cursor))?;
        }

        let slab = Slab::new(start, self.slab_size, &mut self.file)?;
        Ok(self.map.entry(start).or_insert(slab))
    }
}

impl<F: Read + Write + Seek> Read for BufFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let cursor = self.cursor;
        let len = {
            let slab = self.fetch_slab(cursor)?;
            slab.uses = slab.uses.saturating_add(1);
            let mut dat = &slab.dat[(cursor - slab.start) as usize..];
            dat.read(buf)?
        };
        self.cursor += len as u64;
        Ok(len)
    }
}

impl<F: Read + Write + Seek> Write for BufFile<F> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let cursor = self.cursor;
        let len = {
            let slab = self.fetch_slab(cursor)?;
            slab.uses = slab.uses.saturating_add(1);
            slab.dirty = true;
            let mut dat = &mut slab.dat[(cursor - slab.start) as usize..];
            dat.write(buf)?
        };
        self.cursor += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> Result<(), Error> {
        for slab in self.map.values_mut() {
            slab.write(&mut self.file)?;
        }
        Ok(())
    }
}

impl<F: Read + Write + Seek> Seek for BufFile<F> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        match pos {
            SeekFrom::Start(x) => {
                self.fetch_slab(x)?;
                self.cursor = x;
                Ok(x)
            },
            SeekFrom::End(x) => {
                let cursor =
                    if x < 0 { self.end - (-x) as u64 }     // This would be an error if buffers / files
                    else { self.end - x as u64 };           // weren't automatically extended beyond
                                                            // the end.
                self.fetch_slab(cursor)?;
                self.cursor = cursor;
                Ok(cursor)
            },
            SeekFrom::Current(x) => {
                let cur = self.cursor;

                let cursor =
                    if x < 0 { cur - (-x) as u64 }
                    else { cur - x as u64 };
                self.fetch_slab(cursor)?;
                self.cursor = cursor;
                Ok(self.cursor)
            }
        }
    }
}

impl<F: Read + Write + Seek> Drop for BufFile<F> {
     fn drop(&mut self) {
         // Don't panic in drop, so silently ignore any errors.
         let _ = self.flush();
     }
}
