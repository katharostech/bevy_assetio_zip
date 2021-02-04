use std::io::{Read, Seek, Write};

const XOR_FACTOR: u8 = 0b01010101;

/// Reader/Writer implementation that Xor's the bytes that come through it
pub struct Xor<T>(pub T);

impl<R: Read> Read for Xor<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Read bytes into buf
        let count = self.0.read(buf)?;

        // Xor the bytes in the buffer
        for byte in buf {
            *byte = *byte ^ XOR_FACTOR;
        }

        Ok(count)
    }
}

impl<W: Write> Write for Xor<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(
            // Write the buffer with each byte XOR-ed
            buf.iter()
                .map(|x| x ^ XOR_FACTOR)
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl<S: Seek> Seek for Xor<S> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}
