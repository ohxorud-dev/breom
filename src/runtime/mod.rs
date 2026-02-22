use std::alloc::{alloc, dealloc, realloc, Layout};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::Path;
use std::ptr;
use std::sync::atomic::{fence, AtomicI64, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::sync::{mpsc, Arc, Condvar, LazyLock, Mutex};
use std::thread;
use std::time::Duration;

pub mod arc;
pub mod array;
pub mod chan;
pub mod common;
pub mod error;
pub mod file;
pub mod map;
pub mod net;
pub mod sched;
pub mod set;
pub mod string;
pub mod test_api;

#[allow(unused_imports)]
use self::{
    arc::*, array::*, chan::*, common::*, error::*, file::*, map::*, net::*, sched::*, set::*,
    string::*, test_api::*,
};

#[cfg(test)]
mod tests;
