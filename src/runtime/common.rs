use super::*;

pub const CHAN_TYPE_ID: u64 = 5;
pub const SOCKET_TYPE_ID: u64 = 8;
pub const BIND_RESULT_TYPE_ID: u64 = 9;
pub const HTTP_REQUEST_TYPE_ID: u64 = 11;
pub const TCP_SOCKET_TYPE_ID: u64 = 12;
pub const TCP_PACKET_TYPE_ID: u64 = 13;
pub const FILE_READER_TYPE_ID: u64 = 14;
pub const FILE_SCANNER_TYPE_ID: u64 = 15;

pub static SELECT_CONDVAR: Condvar = Condvar::new();
pub static SELECT_MUTEX: Mutex<()> = Mutex::new(());
pub static SELECT_EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub struct TestRuntimeState {
    pub current_name: String,
    pub failures: i64,
}

pub static TEST_STATE: OnceLock<Mutex<TestRuntimeState>> = OnceLock::new();

pub struct BreomChannel {
    pub sender: mpsc::SyncSender<i64>,
    pub receiver: Mutex<mpsc::Receiver<i64>>,
}

pub struct BreomSocket {
    pub socket: UdpSocket,
}

pub struct BreomTcpSocket {
    pub id: i64,
}

pub struct BreomReader {
    pub path: String,
    pub closed: i64,
}

pub struct BreomScanner {
    pub reader: Option<BufReader<fs::File>>,
    pub buffered_line: String,
    pub has_buffered_line: i64,
    pub eof: i64,
    pub closed: i64,
}

#[repr(C)]
pub struct TcpPacketData {
    pub conn: i64,
    pub data: *mut u8,
}

pub struct TcpListenerState {
    pub connections: HashMap<i64, Arc<Mutex<TcpStream>>>,
}

pub static TCP_LISTENERS: LazyLock<Mutex<HashMap<i64, Arc<Mutex<TcpListenerState>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub static TCP_CLIENT_STREAMS: LazyLock<Mutex<HashMap<i64, Arc<Mutex<TcpStream>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub static TCP_NEXT_SOCKET_ID: AtomicI64 = AtomicI64::new(1);
pub static TCP_NEXT_CONN_ID: AtomicI64 = AtomicI64::new(1);

#[repr(C)]
pub struct BindResultData {
    pub socket: *mut u8,
    pub rx: *mut u8,
}

#[repr(C)]
pub struct ArcHeader {
    pub ref_count: AtomicU64,
    pub type_id: u64,
    pub data_size: u64,
}

impl ArcHeader {
    pub const SIZE: usize = std::mem::size_of::<ArcHeader>();
    pub const ALIGN: usize = std::mem::align_of::<ArcHeader>();
}

pub const STRING_TYPE_ID: u64 = 1;

#[repr(C)]
pub struct StringHeader {
    pub len: u64,
}

impl StringHeader {
    pub const SIZE: usize = std::mem::size_of::<StringHeader>();
}

pub const ERROR_TYPE_ID: u64 = 7;

#[repr(C)]
pub struct ErrorData {
    pub message: *mut u8,
}

pub const ARRAY_TYPE_ID: u64 = 2;

#[repr(C)]
pub struct ArrayHeader {
    pub len: u64,
    pub cap: u64,
    pub elem_size: u64,
}

impl ArrayHeader {
    pub const SIZE: usize = std::mem::size_of::<ArrayHeader>();
}

pub const MIN_VALID_HEAP_ADDR: usize = 4096;

#[repr(C)]
pub struct GetCheckedResult {
    pub err: i64,
    pub value: i64,
}

pub const MAP_TYPE_ID: u64 = 3;

#[repr(C)]
pub struct MapEntry {
    pub key_hash: u64,
    pub key: i64,
    pub value: i64,
    pub state: u8,
}

pub const MAP_ENTRY_EMPTY: u8 = 0;
pub const MAP_ENTRY_OCCUPIED: u8 = 1;
pub const MAP_ENTRY_DELETED: u8 = 2;

#[repr(C)]
pub struct MapHeader {
    pub len: u64,
    pub cap: u64,
    pub tombstones: u64,
    pub entries: *mut MapEntry,
}
