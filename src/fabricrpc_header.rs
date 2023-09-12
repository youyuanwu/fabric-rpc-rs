include!(concat!(env!("OUT_DIR"), "/fabricrpc.rs"));

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use prost::Message;

    use super::RequestHeader;

    #[test]
    fn header_test() {
        let mut req = RequestHeader::default();
        req.url = String::from("my url");

        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();

        let req2 = RequestHeader::decode(&mut Cursor::new(buf)).unwrap();

        assert_eq!(req.url, req2.url);
    }
}
