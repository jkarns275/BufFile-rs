use std::io::{ Error, Seek, SeekFrom, Write, Read };
use std::collections::HashMap;
use std::cmp;

/// Slab size MUST be a power of 2!
const SLAB_SIZE: usize = 1024*512; // Change this number to change the SLAB_SIZE (currently @ 512kb)

/// Used to turn a file index into an array index (since SLAB_SIZE is a power of two,
/// subtracting one from it will yield all ones, and anding it with a number will
/// yield only the lowest n bits, where SLAB_SIZE = 2^n
const SLAB_MASK: u64 = SLAB_SIZE as u64 - 1;

const DEFAULT_NUM_SLABS: usize = 16;

/// A struct representing a section of a file
struct Slab {
    /// The data
    pub dat: Vec<u8>,
    /// First byte in the file that is contained in this slab
    start: u64,
    /// Number of times this slab has been accessed.
    uses: u64,
    /// Has the slab been written to, and not written to disk?
    dirty: bool
}

impl Slab {
    /// Creates a new slab, drawing it's data from the given file at the given location
    /// Location should be at the beginning of a slab (e.g. a muitiple of `SLAB_SIZE`)
    pub fn new<F: Seek + Read + Write>(loc: u64, end: u64, file: &mut F) -> Result<Slab, Error> {
        // If loc is greater than the length of the file (e.g. its an invalid Seek) this will return an error
        file.seek(SeekFrom::Start(loc))?;
        let mut dat = vec![0u8; SLAB_SIZE];
        let end = if end - loc >= 2048 { 2048 } else { end & SLAB_MASK };
        file.read(&mut dat[0..end as usize])?;
        Ok(Slab {
            dat: dat,
            start: loc,
            uses: 0,
            dirty: false
        })
    }

    /// Write the slab to disk
    pub fn write<F: Seek + Write>(&mut self, file: &mut F) -> Result<(), Error> {
        if !self.dirty { return Ok(()) }
        file.seek(SeekFrom::Start(self.start))?;
        file.write_all(&self.dat[0..])?;
        self.dirty = false;
        Ok(())
    }
}

pub struct BufFile<F: Write + Read + Seek> {
    /// The maximum number of slabs this BufFile can have
    slabs: usize,
    /// Used to quickly map a file index to an array index (to index self.dat)
    map: HashMap<u64, usize>,
    /// Contains the actual slabs
    dat: Vec<Slab>,
    /// The file to be written to and read from
    file: Option<F>,
    /// Represents the current location of the cursor.
    /// This does not reflect the actual location of the cursor in the file.
    cursor: u64,
    /// The file index that is the end of the file.
    end: u64
}

impl<F: Write + Read + Seek> BufFile<F> {
    /// Creates a new BufFile.
    pub fn new(file: F) -> Result<BufFile<F>, Error> {
        Self::with_capacity(DEFAULT_NUM_SLABS, file)
    }

    /// Creates a new BufFile with the specified number of slabs.
    pub fn with_capacity(slabs: usize, mut file: F) -> Result<BufFile<F>, Error> {
        // Find the end of the file, in case the file isnt empty.
        let end = file.seek(SeekFrom::End(0))?;

        // Move the cursor back to the start of the file.
        file.seek(SeekFrom::Start(0))?;
        Ok(BufFile {
            slabs: slabs,   // Number of slabs
            dat: Vec::with_capacity(slabs),
            map: HashMap::new(),
            file: Some(file),
            cursor: 0,      // Since the cursor is at the start of the file
            end
        })
    }

    /// Returns the underlying Read + Write + Sync struct after writing to disk.
    pub fn into_inner(mut self) -> Result<F, Error> {
        self.flush()?;
        Ok(self.file.take().unwrap())
    }

    /// Change the number of slabs to the desired number. If there are more slabs
    /// currently loaded than `num_slabs`, then the least frequently used slab(s)
    /// will be removed until it is equal. Every removed slab gets written to disk,
    /// creating the possibility for I/O errors.
    pub fn set_slabs(&mut self, num_slabs: usize) -> Result<(), Error> {
        // There isn't anything logical to actually do here, so just return
        if num_slabs == 0 { return Ok(()) }
        if num_slabs >= self.dat.len() {
            self.slabs = num_slabs;
            return Ok(())
        }
        while self.dat.len() > num_slabs {
            let mut min = 0;
            for i in 0..self.slabs {
                if self.dat[min].uses == 1 {
                    min = i;
                    // The minimum number of reads is 1, so if we encounter 1 just break.
                    break;
                }
                if self.dat[min].uses > self.dat[i].uses {
                    min = i;
                }
            }
            self.dat[min].write(self.file.as_mut().unwrap())?;
            let _ = self.dat.swap_remove(min);
        }
        self.slabs = num_slabs;
        Ok(())
    }

    /// Returns the current cursor_loc
    pub fn cursor_loc(&self) -> u64 {
        self.cursor
    }

    /// Finds the slab that contains file index loc, if it doesn't exist None
    /// is returned. If it does exist, Some(index) is returned, where index
    /// is an index into self.dat.
    fn find_slab(&self, loc: u64) -> Option<usize> {
        let start = (loc | SLAB_MASK) ^ SLAB_MASK;
        if self.map.contains_key(&start) {
            let x = self.map[&start].clone();
            Some(x)
        } else {
            None
        }
    }

    /// Adds a slab to the BufFile, if it isn't already present. It will write
    /// the least frequently used slab to disk and load the new one into self.dat,
    /// then return Ok(index), index being an index for self.dat.
    fn add_slab(&mut self, loc: u64) -> Result<usize, Error> {
        let start = (loc | SLAB_MASK) ^ SLAB_MASK;
        if self.map.contains_key(&start) {
            return Ok(self.map[&start].clone());
        }

        // If we're not at the maximum number of slabs, make a new one,
        // and add it to dat and to the map
        if self.dat.len() < self.slabs {
            let ind = self.dat.len();
            match Slab::new(start, self.end, self.file.as_mut().unwrap()) {
                Ok(x) => {
                    self.map.insert(start, self.dat.len());
                    self.dat.push(x);
                    Ok(ind)
                },
                Err(e) => Err(e)
            }
        }

        // We are at the maximum number of slabs - one of them must be removed
        else {
            // Find the minimum - we have to go through all of them, there isn't
            // a simple solution to avoid this that can easily be implemented.
            // (maybe fibonacci heap?)
            let mut min = 0;
            for i in 0..self.slabs {
                if self.dat[min].uses == 1 {
                    min = i;
                    // The minimum number of reads is 1, so if we encounter 1 just break.
                    break;
                }
                if self.dat[min].uses > self.dat[i].uses {
                    min = i;
                }
            }
            // Make a new slab, write the old one to disk, replace old slab
            match Slab::new(start, self.end, self.file.as_mut().unwrap()) {
                Ok(x) => {
                    // Write the old slab to disk
                    self.dat[min].write(self.file.as_mut().unwrap())?;

                    // Move the cursor back to where it was
                    self.file.as_mut().unwrap().seek(SeekFrom::Start(self.cursor))?;

                    // Remove the old slab from the map
                    self.map.remove(&self.dat[min].start);

                    // Add the new one
                    self.map.insert(start, min);

                    // Assign the new value
                    self.dat[min] = x;
                    Ok(min)
                },
                Err(x) => Err(x)
            }
        }
    }
}

impl<F: Write + Read + Seek> Read for BufFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = buf.len();
        // If the place the cursor will be after the read is in the same slab as it will be during the beginning,
        // and the length of the buffer is less than SLAB_SIZE
        if len <= SLAB_SIZE
            && (((len as u64 + self.cursor - 1) | SLAB_MASK) ^ SLAB_MASK == (self.cursor | SLAB_MASK) ^ SLAB_MASK)
            {
            // The index in self.dat (which slab to use)
            let index;
            let cursor = self.cursor;

            // If we dont find it, add a new one!
            match self.find_slab(cursor) {
                Some(x) => index = x,
                None => match self.add_slab(cursor) {
                    Ok(x) => index = x,
                    Err(e) => return Err(e)
                }
            };

            // We're using this slab, so increment its use count
            self.dat[index].uses += 1;
            self.dat[index].dirty = true;
            {
                // Since we're indexing, only use the lower bits n as index.
                let masked = (self.cursor & SLAB_MASK) as usize;
                let slice = &mut self.dat[index].dat[masked as usize .. masked as usize + len];
                buf.clone_from_slice(slice);
            }

            // Move the cursor
            self.cursor += len as u64;

            // Return the number of bytes read
            Ok(len)
        }
        // the data is contained in more than one slab, and may be larger than SLAB_SIZE bytes
        else {
            let mut bytes_read = 0;
            // While we have some reading to do...
            while bytes_read != len {
                // How many bytes to we have to read this iteration? Either the rest of the data or the rest of a slab
                let to_read = cmp::min(SLAB_SIZE - (self.cursor & SLAB_MASK) as usize, len - bytes_read);
                // if cursor is a multiple of SLAB_SIZE then cursor & slab_mask will be 0

                // Which slab to read from
                let index;
                // Combat the borrow-checker
                let cursor = self.cursor;
                // If our slab is already present, cool, if not, add it
                match self.find_slab(cursor) {
                    Some(x) => index = x,
                    None =>
                        match self.add_slab(cursor) {
                            Ok(x) => index = x,
                            Err(e) => return Err(e)
                        }
                };
                // We're using the slab so increment the use count
                self.dat[index].uses += 1;
                self.dat[index].dirty = true;
                {
                    let masked = (self.cursor & SLAB_MASK) as usize;
                    let slice = &mut self.dat[index].dat[masked as usize .. masked as usize + to_read];
                    let mut target = &mut buf[bytes_read .. bytes_read + to_read];
                    target.clone_from_slice(slice);
                }
                self.cursor += to_read as u64;
                bytes_read += to_read;
            }

            Ok(len)
        }
    }
}

impl<F: Write + Read + Seek> Write for BufFile<F> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let len = buf.len();
        // If the place the cursor will be after the write is in the same slab as it will be during the beginning,
        // and the length of the buffer is less than SLAB_SIZE
        if len <= SLAB_SIZE
        && (((len as u64 + self.cursor - 1) | SLAB_MASK) ^ SLAB_MASK == (self.cursor | SLAB_MASK) ^ SLAB_MASK)
        {
            // The index in self.dat (which slab to use)
            let index;
            let cursor = self.cursor;

            // If we dont find it, add a new one!
            match self.find_slab(cursor) {
                Some(x) => index = x,
                None => match self.add_slab(cursor) {
                    Ok(x) => index = x,
                    Err(e) => return Err(e)
                }
            };

            // We're using this slab, so increment its use count
            self.dat[index].uses += 1;
            {
                // Since we're indexing, only use the lower bits n as index.
                let masked = (self.cursor & SLAB_MASK) as usize;
                let mut slice = &mut self.dat[index].dat[masked as usize .. masked as usize + len];
                slice.clone_from_slice(buf);
            }

            // Move the cursor
            self.cursor += len as u64;
            if self.cursor > self.end { self.end = self.cursor }

            // Return the number of bytes written
            Ok(len)
        }
        // the data is contained in more than one slab, and may be larger than SLAB_SIZE bytes
        else {
            let mut bytes_written = 0;
            // While we have some writting to do...
            while bytes_written != len {
                // How many bytes to we have to read this iteration? Either the rest of the data or the rest of a slab
                let to_write = cmp::min(SLAB_SIZE - (self.cursor & SLAB_MASK) as usize, len - bytes_written);
                // if cursor is a multiple of SLAB_SIZE then cursor & slab_mask will be 0

                // Which slab to read from
                let index;
                // Combat the borrow-checker
                let cursor = self.cursor;
                // If our slab is already present, cool, if not, add it
                match self.find_slab(cursor) {
                    Some(x) => index = x,
                    None =>
                        match self.add_slab(cursor) {
                            Ok(x) => index = x,
                            Err(e) => return Err(e)
                        }
                };
                // We're using the slab so increment the use count
                self.dat[index].uses += 1;
                {
                    let masked = (self.cursor & SLAB_MASK) as usize;
                    let mut slice = &mut self.dat[index].dat[masked as usize .. masked as usize + to_write];
                    let target = &buf[bytes_written .. bytes_written + to_write];
                    slice.clone_from_slice(target);
                }
                self.cursor += to_write as u64;
                bytes_written += to_write;
            }

            Ok(len)
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        for slab in self.dat.iter_mut() {
            slab.write(self.file.as_mut().unwrap())?;
        }
        Ok(())
    }
}

impl<F: Write + Read + Seek> Seek for BufFile<F> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        match pos {
            SeekFrom::Start(x) => {
                if self.find_slab(x).is_none() {
                    let cursor = self.cursor;
                    match self.add_slab(cursor) {
                        Ok(_) => {},
                        Err(e) => return Err(e)
                    }
                }
                self.cursor = x;
                Ok(self.cursor)
            },
            SeekFrom::End(x) => {
                self.cursor =
                    if x < 0 { self.end - (-x) as u64 }     // This could be an error if buffers / files
                    else { self.end - x as u64 };           // weren't automatically extended beyond
                                                            // the end.
                let cursor = self.cursor;
                if self.find_slab(cursor).is_none() {
                    match self.add_slab(cursor) {
                        Ok(_) => {},
                        Err(e) => return Err(e)
                    }
                }

                Ok(cursor)
            },
            SeekFrom::Current(x) => {
                let cur = self.cursor;

                let cursor =
                    if x < 0 { cur - (-x) as u64 }
                    else { cur - x as u64 };
                self.cursor = cursor;

                if self.find_slab(cursor).is_none() {
                    match self.add_slab(cursor) {
                        Ok(_) => {},
                        Err(e) => return Err(e)
                    }
                }

                Ok(self.cursor)
            }
        }
    }
}

impl<F: Read + Write + Seek> Drop for BufFile<F> {
    /// Write all of the slabs to disk before closing the file.
     fn drop(&mut self) {
         if self.file.is_none() { return }
         let _ = self.flush();
     }
}
