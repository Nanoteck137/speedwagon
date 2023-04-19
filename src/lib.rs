pub enum Error {
    Test,
}

type Result<T> = std::result::Result<T, Error>;

pub fn test(a: u32) -> Result<()> {
    if a > 10 {
        return Err(Error::Test);
    }

    Ok(())
}
