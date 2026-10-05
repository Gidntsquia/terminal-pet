//! Local-only datagram link between squid-pets: one Unix socket per process in a
//! private (0700) directory. No network. Any setup problem means "run solo".
use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    /// Heartbeat: `away` = squid not at home; `accept` = window big enough for guests.
    Hello {
        away: bool,
        accept: bool,
    },
    Req,
    Ack,
    Nak,
    /// Guest arrives (preset index).
    Arrive(u8),
    Color(u8),
    Recall,
    Home,
    Bye,
    Fish,
    Ink,
    /// Bubble stream; x position as 0..1 of the width.
    Bubbles(f32),
}

impl Msg {
    pub fn encode(&self, pid: u32) -> String {
        match self {
            Msg::Hello { away, accept } => format!("{pid} H {} {}", *away as u8, *accept as u8),
            Msg::Req => format!("{pid} REQ"),
            Msg::Ack => format!("{pid} ACK"),
            Msg::Nak => format!("{pid} NAK"),
            Msg::Arrive(c) => format!("{pid} ARR {c}"),
            Msg::Color(c) => format!("{pid} COL {c}"),
            Msg::Recall => format!("{pid} RCL"),
            Msg::Home => format!("{pid} HOME"),
            Msg::Bye => format!("{pid} BYE"),
            Msg::Fish => format!("{pid} FISH"),
            Msg::Ink => format!("{pid} INK"),
            Msg::Bubbles(x) => format!("{pid} BUB {x:.3}"),
        }
    }

    pub fn decode(s: &str) -> Option<(u32, Msg)> {
        let mut it = s.split_whitespace();
        let pid: u32 = it.next()?.parse().ok()?;
        let tag = it.next()?;
        let a = it.next();
        let b = it.next();
        let m = match tag {
            "H" => Msg::Hello {
                away: a? == "1",
                accept: b? == "1",
            },
            "REQ" => Msg::Req,
            "ACK" => Msg::Ack,
            "NAK" => Msg::Nak,
            "ARR" => Msg::Arrive(a?.parse().ok()?),
            "COL" => Msg::Color(a?.parse().ok()?),
            "RCL" => Msg::Recall,
            "HOME" => Msg::Home,
            "BYE" => Msg::Bye,
            "FISH" => Msg::Fish,
            "INK" => Msg::Ink,
            "BUB" => Msg::Bubbles(a?.parse::<f32>().ok()?.clamp(0.0, 1.0)),
            _ => return None,
        };
        Some((pid, m))
    }
}

pub struct Link {
    sock: UnixDatagram,
    dir: PathBuf,
    path: PathBuf,
    pub pid: u32,
}

fn default_dir() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(x);
        if p.is_absolute() && p.is_dir() {
            return p.join("squid-pet");
        }
    }
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".into());
    let user: String = user
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    PathBuf::from(format!("/tmp/squid-pet-{user}"))
}

impl Link {
    /// Join the local link, or `None` (run solo) if it can't be set up safely.
    pub fn open() -> Option<Link> {
        Link::open_in(&default_dir(), std::process::id())
    }

    pub fn open_in(dir: &Path, pid: u32) -> Option<Link> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .ok()?;
        let dm = fs::metadata(dir).ok()?;
        if !dm.is_dir() || dm.mode() & 0o077 != 0 {
            return None;
        }
        let path = dir.join(format!("{pid}.sock"));
        let _ = fs::remove_file(&path);
        let sock = UnixDatagram::bind(&path).ok()?;
        let ok = sock.set_nonblocking(true).is_ok()
            && fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).is_ok()
            && fs::metadata(&path).is_ok_and(|m| m.uid() == dm.uid());
        let link = Link {
            sock,
            dir: dir.to_path_buf(),
            path,
            pid,
        };
        // On failure `link` drops here and removes the socket file.
        ok.then_some(link)
    }

    fn peer_path(&self, pid: u32) -> PathBuf {
        self.dir.join(format!("{pid}.sock"))
    }

    pub fn send(&self, to: u32, msg: &Msg) {
        let p = self.peer_path(to);
        if p == self.path {
            return;
        }
        match self.sock.send_to(msg.encode(self.pid).as_bytes(), &p) {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
                ) =>
            {
                // Left over from a dead process.
                let _ = fs::remove_file(&p);
            }
            _ => {}
        }
    }

    pub fn broadcast(&self, msg: &Msg) {
        let Ok(rd) = fs::read_dir(&self.dir) else {
            return;
        };
        for e in rd.flatten() {
            let name = e.file_name();
            let Some(pid) = name
                .to_str()
                .and_then(|n| n.strip_suffix(".sock"))
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            if pid != self.pid {
                self.send(pid, msg);
            }
        }
    }

    pub fn recv(&self) -> Vec<(u32, Msg)> {
        let mut out = vec![];
        let mut buf = [0u8; 256];
        while let Ok(n) = self.sock.recv(&mut buf) {
            if let Some(m) = std::str::from_utf8(&buf[..n]).ok().and_then(Msg::decode) {
                out.push(m);
            }
            if out.len() > 512 {
                break;
            }
        }
        out
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("squid-pet-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn msgs_round_trip() {
        for m in [
            Msg::Hello {
                away: true,
                accept: false,
            },
            Msg::Req,
            Msg::Ack,
            Msg::Nak,
            Msg::Arrive(7),
            Msg::Color(3),
            Msg::Recall,
            Msg::Home,
            Msg::Bye,
            Msg::Fish,
            Msg::Ink,
        ] {
            assert_eq!(Msg::decode(&m.encode(42)), Some((42, m)));
        }
        assert!(
            matches!(Msg::decode("5 BUB 0.250"), Some((5, Msg::Bubbles(x))) if (x - 0.25).abs() < 1e-3)
        );
        assert_eq!(Msg::decode("garbage"), None);
        assert_eq!(Msg::decode("1 ARR x"), None);
    }

    #[test]
    fn two_links_talk_and_dead_socket_is_pruned() {
        let d = tmpdir("talk");
        let a = Link::open_in(&d, 1001).unwrap();
        let b = Link::open_in(&d, 1002).unwrap();
        assert_eq!(fs::metadata(&d).unwrap().mode() & 0o777, 0o700);
        a.broadcast(&Msg::Req);
        assert_eq!(b.recv(), vec![(1001, Msg::Req)]);
        b.send(1001, &Msg::Ack);
        assert_eq!(a.recv(), vec![(1002, Msg::Ack)]);
        // Simulate kill -9: socket closed, file left behind.
        let stale = d.join("1003.sock");
        drop(UnixDatagram::bind(&stale).unwrap());
        assert!(stale.exists());
        a.broadcast(&Msg::Bye);
        assert!(!stale.exists(), "stale file should be removed");
        drop(a);
        drop(b);
        assert!(!d.join("1001.sock").exists() && !d.join("1002.sock").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn open_refuses_loose_dir() {
        let d = tmpdir("loose");
        fs::create_dir_all(&d).unwrap();
        fs::set_permissions(&d, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(Link::open_in(&d, 2001).is_none());
        let _ = fs::remove_dir_all(&d);
    }
}
