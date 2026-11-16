//! Windows media session data for ESC Music Controller.
//! Reference: AntiJOPA's ESC Music Controller 1.0.0 (MIT).
use std::{sync::Arc, time::Instant};

pub(super) const ID: &str = "atlas-esc-music";

pub(super) struct Track {
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub playing: bool,
    pub sampled_at: Instant,
    pub cover_rgba: Option<Arc<[u8]>>,
}

#[derive(Clone, Copy)]
pub(super) enum Command {
    Previous,
    Toggle,
    Next,
    Seek(u64),
}

#[derive(Default)]
pub(super) struct State {
    track: Option<Track>,
    #[cfg(windows)]
    worker: Option<platform::Worker>,
}

impl State {
    pub(super) fn poll(&mut self) {
        #[cfg(windows)]
        {
            if self.worker.is_none() {
                self.worker = platform::Worker::start();
            }
            if let Some(worker) = &mut self.worker {
                worker.poll(&mut self.track);
            }
        }
    }

    pub(super) fn track(&self) -> Option<&Track> {
        self.track.as_ref()
    }

    pub(super) fn command(&self, command: Command) {
        #[cfg(windows)]
        if let (Some(worker), Some(track)) = (&self.worker, &self.track) {
            worker.command(command, track.source_id.clone());
        }
        #[cfg(not(windows))]
        let _ = command;
    }
}

#[cfg(windows)]
mod platform {
    use super::{Command, Track};
    use std::{
        sync::{Arc, mpsc},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    use windows::{
        Graphics::Imaging::{
            BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapTransform,
            ColorManagementMode, ExifOrientationMode,
        },
        Media::Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as Manager,
            GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
        },
    };

    enum Request {
        Refresh,
        Action(Command, String),
    }

    pub(super) struct Worker {
        requests: mpsc::SyncSender<Request>,
        results: mpsc::Receiver<Option<Track>>,
        next_refresh: Instant,
    }

    impl Worker {
        pub(super) fn start() -> Option<Self> {
            let (requests, input) = mpsc::sync_channel(8);
            let (output, results) = mpsc::sync_channel(1);
            std::thread::Builder::new()
                .name("atlas-media-session".into())
                .spawn(move || run(input, output))
                .ok()?;
            Some(Self {
                requests,
                results,
                next_refresh: Instant::now(),
            })
        }

        pub(super) fn poll(&mut self, track: &mut Option<Track>) {
            while let Ok(update) = self.results.try_recv() {
                *track = update;
            }
            let now = Instant::now();
            if now >= self.next_refresh {
                let _ = self.requests.try_send(Request::Refresh);
                self.next_refresh = now + Duration::from_millis(300);
            }
        }

        pub(super) fn command(&self, command: Command, source_id: String) {
            let _ = self.requests.try_send(Request::Action(command, source_id));
        }
    }

    fn run(input: mpsc::Receiver<Request>, output: mpsc::SyncSender<Option<Track>>) {
        let mut manager = None;
        let mut cover = None;
        while let Ok(request) = input.recv() {
            if manager.is_none() {
                manager = Manager::RequestAsync()
                    .and_then(|pending| pending.join())
                    .ok();
            }
            let session = manager
                .as_ref()
                .and_then(|manager| manager.GetCurrentSession().ok());
            if let (Request::Action(command, source_id), Some(session)) = (&request, &session)
                && session
                    .SourceAppUserModelId()
                    .is_ok_and(|id| id.to_string() == *source_id)
            {
                let result = match command {
                    Command::Previous => session.TrySkipPreviousAsync(),
                    Command::Toggle => session.TryTogglePlayPauseAsync(),
                    Command::Next => session.TrySkipNextAsync(),
                    Command::Seek(ms) => session.TryChangePlaybackPositionAsync(
                        ms.saturating_mul(10_000).min(i64::MAX as u64) as i64,
                    ),
                };
                let _ = result.and_then(|pending| pending.join());
            }
            let track = session
                .as_ref()
                .map(|session| snapshot(session, &mut cover));
            if track.is_none() {
                cover = None;
            }
            let _ = output.try_send(track);
        }
    }

    struct Cover {
        key: [String; 4],
        pixels: Option<Arc<[u8]>>,
    }

    fn snapshot(session: &Session, cache: &mut Option<Cover>) -> Track {
        let props = session
            .TryGetMediaPropertiesAsync()
            .and_then(|pending| pending.join())
            .ok();
        let title = props
            .as_ref()
            .and_then(|value| value.Title().ok())
            .unwrap_or_default()
            .to_string();
        let artist = props
            .as_ref()
            .and_then(|value| value.Artist().ok())
            .unwrap_or_default()
            .to_string();
        let key = [
            session
                .SourceAppUserModelId()
                .unwrap_or_default()
                .to_string(),
            title.clone(),
            artist.clone(),
            props
                .as_ref()
                .and_then(|value| value.AlbumTitle().ok())
                .unwrap_or_default()
                .to_string(),
        ];
        if cache.as_ref().is_none_or(|cover| cover.key != key) {
            *cache = Some(Cover {
                key,
                pixels: props.as_ref().and_then(decode_cover),
            });
        }
        let playing = session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            == Ok(PlaybackStatus::Playing);
        let (position_ms, duration_ms) =
            session
                .GetTimelineProperties()
                .ok()
                .map_or((0, 0), |timeline| {
                    let duration = timeline
                        .EndTime()
                        .map_or(0, |time| time.Duration.max(0) as u64 / 10_000);
                    let mut position = timeline
                        .Position()
                        .map_or(0, |time| time.Duration.max(0) as u64 / 10_000);
                    if playing
                        && let Ok(updated) = timeline.LastUpdatedTime()
                        && updated.UniversalTime > 0
                    {
                        // WinRT timestamps count 100ns units from 1601, Unix timestamps from 1970.
                        let unix_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let now_ms = unix_ms + 11_644_473_600_000;
                        position = position.saturating_add(
                            now_ms.saturating_sub(updated.UniversalTime as u64 / 10_000),
                        );
                    }
                    (
                        if duration > 0 {
                            position.min(duration)
                        } else {
                            position
                        },
                        duration,
                    )
                });
        Track {
            source_id: session
                .SourceAppUserModelId()
                .unwrap_or_default()
                .to_string(),
            title,
            artist,
            duration_ms,
            position_ms,
            playing,
            sampled_at: Instant::now(),
            cover_rgba: cache.as_ref().and_then(|cover| cover.pixels.clone()),
        }
    }

    fn decode_cover(props: &Properties) -> Option<Arc<[u8]>> {
        let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
        if stream.Size().ok()? > 8 * 1024 * 1024 {
            return None;
        }
        let decoder = BitmapDecoder::CreateAsync(&stream).ok()?.join().ok()?;
        if decoder.PixelWidth().ok()? > 4096 || decoder.PixelHeight().ok()? > 4096 {
            return None;
        }
        let transform = BitmapTransform::new().ok()?;
        transform.SetScaledWidth(64).ok()?;
        transform.SetScaledHeight(64).ok()?;
        let data = decoder
            .GetPixelDataTransformedAsync(
                BitmapPixelFormat::Rgba8,
                BitmapAlphaMode::Straight,
                &transform,
                ExifOrientationMode::IgnoreExifOrientation,
                ColorManagementMode::DoNotColorManage,
            )
            .ok()?
            .join()
            .ok()?
            .DetachPixelData()
            .ok()?;
        (data.len() == 64 * 64 * 4).then(|| Arc::from(&data[..]))
    }
}
