extern crate batch;
extern crate batch_rabbitmq;
extern crate serde;

pub mod queues {
    use batch_rabbitmq::queues;

    queues! {
        Transcoding {
            name = "transcoding",
            bindings = [
                super::jobs::convert_video_file
            ]
        }
    }
}

pub mod jobs {
    use batch::export::Error;
    use batch::job;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum VideoFormat {
        Matroska,
        Mpeg4,
    }

    #[job(name = "batch-example.convert-video-file")]
    pub fn convert_video_file(path: String, to: VideoFormat) {
        println!("Converting video file {:?} to {:?}...", path, to);
    }

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        ConvertStarted(String),
    }

    pub type Mail = ();

    #[job(name = "batch-example.notify", inject = [ _mailer ])]
    pub fn notify(_mailer: Mail, _user: String, _event: String) -> Result<(), Error> {
        println!("Notifying user...");
        Ok(())
    }
}
