pub struct NBTSizeTracker {
    max: i64,
    read: i64
}

impl NBTSizeTracker {
    pub fn new(max: i64) -> NBTSizeTracker {
        NBTSizeTracker {
            max,
            read: 0
        }
    }
    
    pub fn read(&mut self, bits: i64) -> anyhow::Result<()> {
        self.read += bits / 8;
        if self.read > self.max {
            return Err(anyhow::anyhow!("NBT too large"));
        }
        Ok(())
    }
}