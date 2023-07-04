use actix_web::HttpRequest;

pub fn full_uri(request: &HttpRequest) -> String {
    let connection_info = request.connection_info();
    let scheme = connection_info.scheme();
    let host = connection_info.host();
    let relative_uri = request.uri();

    format!("{}://{}{}", scheme, host, relative_uri)
}
