extern crate hubcaps;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate uritemplate;
extern crate mime;

use std::io::Read;

use url::Url;
use uritemplate::UriTemplate;
use serde::de::Deserialize;

use hyper::Client;
use hyper::client::RequestBuilder;
use hyper::method::Method;
use hyper::header::{Authorization, ContentLength, UserAgent, ContentType};
use hyper::status::StatusCode;

use hubcaps::Credentials;
use hubcaps::errors::Error;
use hubcaps::rep::{Release, Asset, ClientError};

const USERAGENT : &'static str = concat!("hubcaps-assets-upload/", env!("CARGO_PKG_VERSION"));

pub struct AssetsClient<'a> {
    agent: String,
    client: Client,
    credentials: &'a Credentials,
}

impl<'a> AssetsClient<'a> {
    pub fn new(credentials: &'a Credentials) -> AssetsClient<'a> {
        AssetsClient {
            agent: USERAGENT.into(),
            client: Client::new(),
            credentials: credentials
        }
    }

    fn authenticate(&self, method: Method, mut url: Url) -> RequestBuilder {
        match self.credentials {
            &Credentials::Token(ref token) => {
                self.client.request(method, url).header(Authorization(format!("token {}", token)))
            }
            &Credentials::Client(ref id, ref secret) => {
                let mut query = url.query_pairs().unwrap_or(vec![]);
                query.push(("client_id".to_owned(), id.to_owned()));
                query.push(("client_secret".to_owned(), secret.to_owned()));
                url.set_query_from_pairs(query);
                self.client.request(method, url)
            }
            &Credentials::None => self.client.request(method, url),
        }
    }

    fn post<D>(&self, uri: Url, content_type: mime::Mime, body: &[u8]) -> hubcaps::Result<D>
        where D: Deserialize
    {
        let builder = self.authenticate(Method::Post, uri)
            .header(UserAgent(self.agent.to_owned()))
            .header(ContentType(content_type));

        let mut res = try!(builder.body(body).send());

        let mut body = match res.headers.clone().get::<ContentLength>() {
            Some(&ContentLength(len)) => String::with_capacity(len as usize),
            _ => String::new(),
        };
        try!(res.read_to_string(&mut body));
        match res.status {
            StatusCode::Conflict |
            StatusCode::BadRequest |
            StatusCode::UnprocessableEntity |
            StatusCode::Unauthorized |
            StatusCode::NotFound |
            StatusCode::Forbidden => {
                Err(Error::Fault {
                    code: res.status,
                    error: try!(serde_json::from_str::<ClientError>(&body)),
                })
            }
            _ => Ok(try!(serde_json::from_str::<D>(&body))),
        }
    }
}

pub struct AssetRequest<R> {
    name: String,
    content_type: mime::Mime,
    label: Option<String>,
    content: Option<R>
}

impl<R: Read> AssetRequest<R> {
    pub fn new<N: Into<String>>(name: N, content_type: mime::Mime, label: Option<String>) -> AssetRequest<R> {
        AssetRequest {
            name: name.into(),
            content_type: content_type,
            label: label,
            content: None,
        }
    }

    pub fn content(&mut self, c: R) {
        self.content = Some(c);
    }
}

pub struct AssetUploader {
    credentials: Credentials
}

impl AssetUploader {
    pub fn new(credentials: Credentials) -> AssetUploader {
        AssetUploader {
            credentials: credentials
        }
    }

    pub fn upload<R: Read>(&self, release: &Release, asset_req: AssetRequest<R>) -> hubcaps::Result<Asset> {
        assert!(asset_req.content.is_some());

        let mut url_tmpl = UriTemplate::new(&release.upload_url);
        url_tmpl.set("name", asset_req.name);
        if let Some(label) = asset_req.label {
            url_tmpl.set("label", label);
        }

        let url = url_tmpl.build();
        let url = Url::parse(&url).expect("Valid URL");

        let client = AssetsClient::new(&self.credentials);

        let mut buf = Vec::new();
        asset_req.content.unwrap().read_to_end(&mut buf).expect("Read failed.");
        client.post(url, asset_req.content_type, &buf)
    }
}


#[cfg(test)]
mod test {
    #[test]
    fn it_works() {
    }
}
