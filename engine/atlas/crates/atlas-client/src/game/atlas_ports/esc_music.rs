//! Windows media session data for ESC Music Controller.
//! Reference: AntiJOPA's ESC Music Controller 1.0.0 (MIT).
use atlas_client::hud::pictures::Picture;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) const ID: &str = "atlas-esc-music";

pub(super) struct Track {
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub playing: bool,
    pub repeat_one: bool,
    pub can_repeat: bool,
    pub can_seek: bool,
    pub(super) native_repeat: bool,
    pub sampled_at: Instant,
    pub cover: Option<Arc<Picture>>,
}

#[derive(Clone, Copy)]
pub(super) enum Command {
    Previous,
    Toggle,
    RepeatOne,
    Next,
    Seek(u64),
}

#[derive(Default)]
pub(super) struct State {
    track: Option<Track>,
    pending_play: Option<PendingPlay>,
    next_request: u64,
    #[cfg(windows)]
    worker: Option<platform::Worker>,
}

struct PendingPlay {
    request: u64,
    playing: bool,
    acknowledged: bool,
    expires: Instant,
}

impl PendingPlay {
    fn awaiting(&self, track: &Track, now: Instant) -> bool {
        (!self.acknowledged || self.playing != track.playing) && now < self.expires
    }
}

fn near_track_end(track: &Track) -> bool {
    track.playing
        && track.can_seek
        && track.duration_ms >= 2000
        && track.duration_ms.saturating_sub(track.position_ms) <= 160
}

impl State {
    pub(super) fn poll(&mut self) {
        #[cfg(windows)]
        {
            if self.worker.is_none() {
                self.worker = platform::Worker::start();
            }
            if let Some(worker) = &mut self.worker {
                for update in worker.poll() {
                    let same_session = self
                        .track
                        .as_ref()
                        .zip(update.track.as_ref())
                        .is_some_and(|(old, new)| old.source_id == new.source_id);
                    if !same_session
                        || self.pending_play.as_mut().is_some_and(|pending| {
                            pending.acknowledged |= update.completed == Some(pending.request);
                            update.rejected == Some(pending.request)
                                || update
                                    .track
                                    .as_ref()
                                    .is_some_and(|track| !pending.awaiting(track, Instant::now()))
                        })
                    {
                        self.pending_play = None;
                    }
                    self.track = update.track;
                }
            }
        }
    }

    pub(super) fn track(&self) -> Option<&Track> {
        self.track.as_ref()
    }

    pub(super) fn playing(&self) -> bool {
        self.track.as_ref().is_some_and(|track| {
            self.pending_play
                .as_ref()
                .filter(|pending| pending.awaiting(track, Instant::now()))
                .map_or(track.playing, |pending| pending.playing)
        })
    }

    pub(super) fn command(&mut self, command: Command) {
        #[cfg(windows)]
        if let (Some(worker), Some(track)) = (&self.worker, &self.track) {
            let desired = matches!(command, Command::Toggle).then(|| !self.playing());
            let request = self.next_request;
            self.next_request = self.next_request.wrapping_add(1);
            if worker.command(command, track.source_id.clone(), desired, request)
                && let Some(playing) = desired
            {
                self.pending_play = Some(PendingPlay {
                    request,
                    playing,
                    acknowledged: false,
                    expires: Instant::now() + Duration::from_millis(800),
                });
            }
        }
        #[cfg(not(windows))]
        let _ = command;
    }
}

#[cfg(windows)]
mod platform {
    use super::{Command, Track, near_track_end};
    use atlas_client::hud::{TextureFlags, pictures::Picture};
    use std::{
        sync::{Arc, mpsc},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    use windows::{
        Graphics::Imaging::{
            BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapTransform,
            ColorManagementMode, ExifOrientationMode,
        },
        Media::{
            Control::{
                GlobalSystemMediaTransportControlsSession as Session,
                GlobalSystemMediaTransportControlsSessionManager as Manager,
                GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
                GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
            },
            MediaPlaybackAutoRepeatMode as RepeatMode,
        },
    };

    enum Request {
        Refresh,
        Action(Command, String, Option<bool>, u64),
    }

    pub(super) struct Update {
        pub(super) track: Option<Track>,
        pub(super) completed: Option<u64>,
        pub(super) rejected: Option<u64>,
    }

    pub(super) struct Worker {
        requests: mpsc::SyncSender<Request>,
        results: mpsc::Receiver<Update>,
        next_refresh: Instant,
    }

    impl Worker {
        pub(super) fn start() -> Option<Self> {
            let (requests, input) = mpsc::sync_channel(8);
            let (output, results) = mpsc::channel();
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

        pub(super) fn poll(&mut self) -> impl Iterator<Item = Update> + '_ {
            let now = Instant::now();
            if now >= self.next_refresh {
                let _ = self.requests.try_send(Request::Refresh);
                self.next_refresh = now + Duration::from_millis(150);
            }
            self.results.try_iter()
        }

        pub(super) fn command(
            &self,
            command: Command,
            source_id: String,
            desired: Option<bool>,
            request: u64,
        ) -> bool {
            self.requests
                .try_send(Request::Action(command, source_id, desired, request))
                .is_ok()
        }
    }

    struct LocalRepeat {
        key: [String; 4],
        armed: bool,
    }

    fn run(input: mpsc::Receiver<Request>, output: mpsc::Sender<Update>) {
        let mut manager = None;
        let mut cover = None;
        let mut repeat: Option<LocalRepeat> = None;
        loop {
            let request = if repeat.is_some() {
                match input.recv_timeout(Duration::from_millis(75)) {
                    Ok(request) => Some(request),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            } else {
                let Ok(request) = input.recv() else { break };
                Some(request)
            };
            if manager.is_none() {
                manager = Manager::RequestAsync()
                    .and_then(|pending| pending.join())
                    .ok();
            }
            let session = manager
                .as_ref()
                .and_then(|manager| manager.GetCurrentSession().ok());
            let mut completed = None;
            let mut rejected = None;
            if let Some(Request::Action(command, source_id, desired, id)) = request.as_ref() {
                if let Some(session) = &session
                    && session
                        .SourceAppUserModelId()
                        .is_ok_and(|source| source.to_string() == *source_id)
                {
                    match command {
                        Command::Toggle => {
                            let playing = session
                                .GetPlaybackInfo()
                                .and_then(|info| info.PlaybackStatus())
                                == Ok(PlaybackStatus::Playing);
                            if play_or_pause(session, desired.unwrap_or(!playing), playing) {
                                completed = Some(*id);
                            } else {
                                rejected = Some(*id);
                            }
                        }
                        Command::Previous => {
                            let _ = session.TrySkipPreviousAsync().and_then(|op| op.join());
                        }
                        Command::Next => {
                            let _ = session.TrySkipNextAsync().and_then(|op| op.join());
                        }
                        Command::Seek(ms) => {
                            let _ = seek(session, *ms);
                        }
                        Command::RepeatOne => {}
                    }
                } else if matches!(command, Command::Toggle) {
                    rejected = Some(*id);
                }
            }
            let mut track = session
                .as_ref()
                .map(|session| snapshot(session, &mut cover));
            if track.is_none() {
                cover = None;
            }
            if repeat
                .as_ref()
                .is_some_and(|repeat| cover.as_ref().is_none_or(|cover| cover.key != repeat.key))
            {
                repeat = None;
            }
            if let Some(Request::Action(Command::RepeatOne, source_id, _, _)) = request.as_ref()
                && let (Some(session), Some(current)) = (&session, track.as_mut())
                && current.source_id == *source_id
            {
                let was_local = repeat.take().is_some();
                let enable = !was_local && !current.repeat_one;
                let mode = if enable {
                    RepeatMode::Track
                } else {
                    RepeatMode::None
                };
                let native = current.native_repeat
                    && (!was_local || current.repeat_one)
                    && session
                        .TryChangeAutoRepeatModeAsync(mode)
                        .and_then(|op| op.join())
                        == Ok(true);
                if native {
                    current.repeat_one = enable;
                } else if enable
                    && current.can_seek
                    && current.duration_ms >= 2000
                    && !current.title.is_empty()
                    && let Some(cover) = &cover
                {
                    repeat = Some(LocalRepeat {
                        key: cover.key.clone(),
                        armed: true,
                    });
                }
            }
            if let (Some(repeat), Some(session), Some(track)) = (&mut repeat, &session, &mut track)
            {
                if track.duration_ms.saturating_sub(track.position_ms) > 500 {
                    repeat.armed = true;
                }
                if repeat.armed
                    && near_track_end(track)
                    && !matches!(request, Some(Request::Action(Command::Toggle, ..)))
                {
                    repeat.armed = false;
                    if seek(session, 0) {
                        track.position_ms = 0;
                        track.sampled_at = Instant::now();
                    }
                }
                track.repeat_one = true;
            }
            if request.is_some()
                && output
                    .send(Update {
                        track,
                        completed,
                        rejected,
                    })
                    .is_err()
            {
                break;
            }
        }
    }

    fn seek(session: &Session, ms: u64) -> bool {
        session
            .TryChangePlaybackPositionAsync(ms.saturating_mul(10_000).min(i64::MAX as u64) as i64)
            .and_then(|op| op.join())
            == Ok(true)
    }

    fn play_or_pause(session: &Session, play: bool, playing: bool) -> bool {
        if play == playing {
            return true;
        }
        let direct = if play {
            session.TryPlayAsync()
        } else {
            session.TryPauseAsync()
        };
        direct.and_then(|op| op.join()) == Ok(true)
            || session.TryTogglePlayPauseAsync().and_then(|op| op.join()) == Ok(true)
    }

    struct Cover {
        key: [String; 4],
        picture: Option<Arc<Picture>>,
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
                picture: props.as_ref().and_then(decode_cover),
            });
        }
        let info = session.GetPlaybackInfo().ok();
        let playing = info.as_ref().and_then(|info| info.PlaybackStatus().ok())
            == Some(PlaybackStatus::Playing);
        let controls = info.as_ref().and_then(|info| info.Controls().ok());
        let can_seek = controls
            .as_ref()
            .is_some_and(|controls| controls.IsPlaybackPositionEnabled() == Ok(true));
        let native_repeat = controls
            .as_ref()
            .is_some_and(|controls| controls.IsRepeatEnabled() == Ok(true));
        let repeat_one = info
            .as_ref()
            .and_then(|info| info.AutoRepeatMode().ok())
            .and_then(|mode| mode.Value().ok())
            == Some(RepeatMode::Track);
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
        let can_repeat = native_repeat || (can_seek && duration_ms >= 2000 && !title.is_empty());
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
            repeat_one,
            can_repeat,
            can_seek,
            native_repeat,
            sampled_at: Instant::now(),
            cover: cache.as_ref().and_then(|cover| cover.picture.clone()),
        }
    }

    fn decode_cover(props: &Properties) -> Option<Arc<Picture>> {
        const SIZE: u32 = 160;
        let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
        if stream.Size().ok()? > 8 * 1024 * 1024 {
            return None;
        }
        let decoder = BitmapDecoder::CreateAsync(&stream).ok()?.join().ok()?;
        if decoder.PixelWidth().ok()? > 4096 || decoder.PixelHeight().ok()? > 4096 {
            return None;
        }
        let transform = BitmapTransform::new().ok()?;
        transform.SetScaledWidth(SIZE).ok()?;
        transform.SetScaledHeight(SIZE).ok()?;
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
        let mut pixels = data.to_vec();
        let side = SIZE as f32;
        let radius = side * (7.0 / 24.0);
        for (i, pixel) in pixels.chunks_exact_mut(4).enumerate() {
            let x = (i % SIZE as usize) as f32 + 0.5;
            let y = (i / SIZE as usize) as f32 + 0.5;
            let dx = (radius - x.min(side - x)).max(0.0);
            let dy = (radius - y.min(side - y)).max(0.0);
            let coverage = (radius + 0.5 - dx.hypot(dy)).clamp(0.0, 1.0);
            pixel[3] = (f32::from(pixel[3]) * coverage).round() as u8;
        }
        Picture::rgba(
            SIZE,
            SIZE,
            pixels,
            TextureFlags {
                blur: true,
                clamp: true,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(playing: bool) -> Track {
        Track {
            source_id: String::new(),
            title: String::new(),
            artist: String::new(),
            duration_ms: 60_000,
            position_ms: 59_940,
            playing,
            repeat_one: false,
            can_repeat: true,
            can_seek: true,
            native_repeat: false,
            sampled_at: Instant::now(),
            cover: None,
        }
    }

    #[test]
    fn matching_older_playback_does_not_confirm_a_queued_toggle() {
        let now = Instant::now();
        let pending = PendingPlay {
            request: 1,
            playing: false,
            acknowledged: false,
            expires: now + Duration::from_secs(1),
        };
        assert!(pending.awaiting(&track(false), now));
        let confirmed = PendingPlay {
            acknowledged: true,
            ..pending
        };
        assert!(!confirmed.awaiting(&track(false), now));
        assert!(confirmed.awaiting(&track(true), now));
        assert!(!confirmed.awaiting(&track(true), now + Duration::from_secs(1)));
    }

    #[test]
    fn software_repeat_never_resumes_a_paused_or_unseekable_track() {
        let mut track = track(true);
        assert!(near_track_end(&track));
        track.playing = false;
        assert!(!near_track_end(&track));
        track.playing = true;
        track.can_seek = false;
        assert!(!near_track_end(&track));
    }
}
