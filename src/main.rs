extern crate hubcaps;
extern crate hubcaps_asset_upload;
extern crate hyper;
extern crate env_logger;

use std::path::Path;
use std::{env, process};
use std::io::BufReader;
use std::fs::File;

use hubcaps_asset_upload::{AssetRequest, AssetUploader};
use hyper::Client;
use hubcaps::{Github, Credentials};

fn main() {
    env_logger::init().expect("Can't instantiate env logger");

    let token = match env::var("GH_TOKEN") {
        Ok(token) => token,
        _ => {
            println!("example missing GH_TOKEN");
            process::exit(1);

        }

    };

    let client = Client::new();
    let credentials = Credentials::Token(token.clone());
    let credentials2 = Credentials::Token(token);
    let github = Github::new(
        format!("hubcaps/{}", env!("CARGO_PKG_VERSION")),
        &client,
        credentials2);

    let mut args = env::args().skip(1);

    let user = args.next().expect("User argument");
    let repo = args.next().expect("Repo argument");
    let path = args.next().expect("File argument");

    let repo = github.repo(user, repo);
    let release = repo.releases();
    let rls = release.list().expect("List of releases");

    let newest_rls = rls.iter().next().unwrap();
    println!("Release: {}", newest_rls.name);
    println!("Date: {}", newest_rls.published_at);
    println!("Upload URL: {}", newest_rls.upload_url);


    let path = Path::new(&path);
    let file = path.file_name().unwrap().to_str().unwrap();
    let f = File::open(path).unwrap();
    let reader = BufReader::new(f);

    let mimetype = "text/plain".parse().unwrap();
    let mut asset_req = AssetRequest::new(file, mimetype, None);
    asset_req.content(reader);

    let uploader = AssetUploader::new(credentials);
    let asset = uploader.upload(newest_rls, asset_req).unwrap();
}
