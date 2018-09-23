extern crate batch;
extern crate batch_rabbitmq;
extern crate serde;

pub mod queues {
    use batch_rabbitmq::queues;

    queues! {
        Transcoding {
            name = "transcoding",
            with_priorities = true,
            exclusive = true,
            bindings = {
                super::exchanges::Transcoding = [
                    super::jobs::convert_video_file
                ]
            }
        }
    }
}

pub mod exchanges {
    use batch_rabbitmq::exchanges;

    exchanges! {
        Transcoding {
            name = "transcoding",
            kind = direct,
        }
    }
}

pub mod jobs {
    use batch::job;
    use serde::{Serialize, Deserialize};

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
