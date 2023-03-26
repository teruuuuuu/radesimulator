

pub trait FrameParser<T> {
    fn parse(src: &mut Cursor<&[u8]>) -> Result<T, Err>;
}

#[test]
fn test() {
    
}