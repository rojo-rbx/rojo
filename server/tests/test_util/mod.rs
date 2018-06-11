use rouille::Request;

use librojo::web::Server;

pub trait HttpTestUtil {
    fn get_string(&self, url: &str) -> String;
}

impl HttpTestUtil for Server {
    fn get_string(&self, url: &str) -> String {
        let info_request = Request::fake_http("GET", url, vec![], vec![]);
        let response = self.handle_request(&info_request);

        assert_eq!(response.status_code, 200);

        let (mut reader, _) = response.data.into_reader_and_size();
        let mut body = String::new();
        reader.read_to_string(&mut body).unwrap();

        body
    }
}
