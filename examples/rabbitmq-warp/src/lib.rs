extern crate batch;
extern crate batch_rabbitmq;
extern crate batch_stub;
#[cfg(test)]
extern crate env_logger;
extern crate futures;
extern crate serde;
#[cfg(test)]
extern crate tokio;
extern crate warp;

pub mod queues {
    use batch_rabbitmq::queues;

    queues! {
        Transcoding {
            name = "transcoding",
            exclusive = true,
            bindings = [
                super::jobs::convert_video_file
            ]
        }
    }
}

pub mod jobs {
    use batch::job;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum VideoFormat {
        Matroska,
        Mpeg4,
    }

    #[job(name = "batch-example.convert-video-file")]
    pub fn convert_video_file(_path: String, _to: VideoFormat) {
        println!("Converting video file...");
    }

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        ConvertStarted(String),
    }

    pub type Mail = ();

    #[job(name = "batch-example.notify", inject = [ _mailer ])]
    pub fn notify(_mailer: Mail, _user: String, _event: Event) {
        println!("Notifying user...");
    }
}

pub mod endpoints {
    use super::{jobs, queues};
    use batch;
    use batch_rabbitmq;
    use futures::Future;
    use warp::{self, Filter, Rejection, Reply};

    fn transcode(
        mut client: impl batch::Client,
    ) -> impl Future<Item = impl Reply, Error = Rejection> {
        let job = jobs::convert_video_file("./westworld-2x06.mkv".into(), jobs::VideoFormat::Mpeg4);
        queues::Transcoding(job)
            .dispatch(&mut client)
            .map(|_| warp::reply())
            .map_err(|_e| warp::reject::server_error())
    }

    pub fn endpoints(
        client: batch_rabbitmq::Connection,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> {
        let client = warp::any().map(move || client.clone());
        warp::get2()
            .and(warp::path("transcode"))
            .and(client)
            .and_then(transcode)
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use batch_stub;
        use env_logger;
        use tokio::runtime::current_thread::Runtime;

        #[test]
        fn test_transcode_endpoint() {
            let _ = env_logger::try_init();
            let mut runtime = Runtime::new().unwrap();
            let client = batch_stub::Client::new();
            let fut = transcode(client.clone());
            let r = runtime.block_on(fut);
            assert!(r.is_ok());
            assert_eq!(1, client.count(queues::Transcoding));
        }
    }
}
