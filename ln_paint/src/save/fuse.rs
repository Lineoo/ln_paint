use std::{
    collections::{BTreeSet, HashMap},
    ffi::OsStr,
    io,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, UNIX_EPOCH},
};

use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    LockOwner, MountOption, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, Request, TimeOrNow, WriteFlags,
};
use redb::{Database, ReadableDatabase, ReadableTable};
use uuid::Uuid;

use crate::save::{
    chunk::{TABLE_LAYER_CHUNK, TABLE_LAYER_CHUNK_RAW},
    object::TABLE_LAYER_OBJECT,
};

const TTL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Table {
    Object,
    Chunk,
    Raw,
}

impl Table {
    const ALL: [Table; 3] = [Table::Object, Table::Chunk, Table::Raw];

    fn name(self) -> &'static str {
        match self {
            Table::Object => "layer_object",
            Table::Chunk => "layer_chunk",
            Table::Raw => "layer_chunk_raw",
        }
    }

    fn from_name(name: &str) -> Option<Table> {
        Table::ALL.into_iter().find(|table| table.name() == name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    Root,
    Table(Table),
    Page { table: Table, page: u64 },
    Object { page: u64, uuid: u128 },
    Chunk { page: u64, x: i32, y: i32, z: u8 },
    Raw { page: u64, x: i32, y: i32 },
}

impl Node {
    fn kind(self) -> FileType {
        match self {
            Node::Object { .. } | Node::Chunk { .. } | Node::Raw { .. } => FileType::RegularFile,
            _ => FileType::Directory,
        }
    }

    fn parent(self) -> Option<Node> {
        match self {
            Node::Root => None,
            Node::Table(_) => Some(Node::Root),
            Node::Page { table, .. } => Some(Node::Table(table)),
            Node::Object { page, .. } => Some(Node::Page {
                table: Table::Object,
                page,
            }),
            Node::Chunk { page, .. } => Some(Node::Page {
                table: Table::Chunk,
                page,
            }),
            Node::Raw { page, .. } => Some(Node::Page {
                table: Table::Raw,
                page,
            }),
        }
    }
}

struct State {
    inos: HashMap<Node, INodeNo>,
    nodes: HashMap<INodeNo, Node>,
    next: u64,
    pages: BTreeSet<u64>,
}

impl State {
    fn intern(&mut self, node: Node) -> INodeNo {
        if let Some(&ino) = self.inos.get(&node) {
            return ino;
        }
        let ino = INodeNo(self.next);
        self.next += 1;
        self.inos.insert(node, ino);
        self.nodes.insert(ino, node);
        ino
    }
}

struct Fuse {
    db: Arc<Database>,
    state: Mutex<State>,
}

impl Fuse {
    fn new(db: Arc<Database>) -> Self {
        let pages = {
            let read = db.begin_read().unwrap();
            let mut pages = BTreeSet::new();
            for entry in read.open_table(TABLE_LAYER_OBJECT).unwrap().iter().unwrap() {
                let (key, _) = entry.unwrap();
                pages.insert(key.value().0);
            }
            for entry in read.open_table(TABLE_LAYER_CHUNK).unwrap().iter().unwrap() {
                let (key, _) = entry.unwrap();
                pages.insert(key.value().0);
            }
            for entry in read
                .open_table(TABLE_LAYER_CHUNK_RAW)
                .unwrap()
                .iter()
                .unwrap()
            {
                let (key, _) = entry.unwrap();
                pages.insert(key.value().0);
            }
            pages
        };

        let mut state = State {
            inos: HashMap::new(),
            nodes: HashMap::new(),
            next: 5,
            pages,
        };
        state.inos.insert(Node::Root, INodeNo::ROOT);
        state.nodes.insert(INodeNo::ROOT, Node::Root);
        for (i, table) in Table::ALL.into_iter().enumerate() {
            let ino = INodeNo(i as u64 + 2);
            state.inos.insert(Node::Table(table), ino);
            state.nodes.insert(ino, Node::Table(table));
        }

        Fuse {
            db,
            state: Mutex::new(state),
        }
    }

    fn node(&self, ino: INodeNo) -> Option<Node> {
        self.state.lock().unwrap().nodes.get(&ino).copied()
    }

    fn object(&self, page: u64, uuid: u128) -> Option<Vec<u8>> {
        let read = self.db.begin_read().unwrap();
        let table = read.open_table(TABLE_LAYER_OBJECT).unwrap();
        let value = table.get((page, uuid)).unwrap()?;
        Some(value.value().to_vec())
    }

    fn set_object(&self, page: u64, uuid: u128, bytes: &[u8]) {
        let write = self.db.begin_write().unwrap();
        {
            let mut table = write.open_table(TABLE_LAYER_OBJECT).unwrap();
            table.insert((page, uuid), bytes).unwrap();
        }
        write.commit().unwrap();
    }

    fn remove_object(&self, page: u64, uuid: u128) -> bool {
        let write = self.db.begin_write().unwrap();
        let existed = {
            let mut table = write.open_table(TABLE_LAYER_OBJECT).unwrap();
            table.remove((page, uuid)).unwrap().is_some()
        };
        write.commit().unwrap();
        existed
    }

    fn chunk_exists(&self, page: u64, x: i32, y: i32, z: u8) -> bool {
        let read = self.db.begin_read().unwrap();
        let table = read.open_table(TABLE_LAYER_CHUNK).unwrap();
        table.get((page, (x, y, z))).unwrap().is_some()
    }

    fn raw_exists(&self, page: u64, x: i32, y: i32) -> bool {
        let read = self.db.begin_read().unwrap();
        let table = read.open_table(TABLE_LAYER_CHUNK_RAW).unwrap();
        table.get((page, (x, y))).unwrap().is_some()
    }

    fn child(&self, parent: INodeNo, name: &str) -> Option<Node> {
        let parent = self.node(parent)?;
        match parent {
            Node::Root => Table::from_name(name).map(Node::Table),
            Node::Table(table) => {
                let page = name.parse().ok()?;
                self.state
                    .lock()
                    .unwrap()
                    .pages
                    .contains(&page)
                    .then_some(Node::Page { table, page })
            }
            Node::Page {
                table: Table::Object,
                page,
            } => {
                let uuid = object_uuid(name)?;
                self.object(page, uuid).map(|_| Node::Object { page, uuid })
            }
            Node::Page {
                table: Table::Chunk,
                page,
            } => {
                let (x, y, z) = chunk_key(name)?;
                self.chunk_exists(page, x, y, z)
                    .then_some(Node::Chunk { page, x, y, z })
            }
            Node::Page {
                table: Table::Raw,
                page,
            } => {
                let (x, y) = raw_key(name)?;
                self.raw_exists(page, x, y)
                    .then_some(Node::Raw { page, x, y })
            }
            _ => None,
        }
    }

    fn entries(&self, node: Node) -> Option<Vec<(Node, String)>> {
        let mut entries = Vec::new();
        match node {
            Node::Root => {
                for table in Table::ALL {
                    entries.push((Node::Table(table), table.name().to_string()));
                }
            }
            Node::Table(table) => {
                for page in self.state.lock().unwrap().pages.iter() {
                    entries.push((Node::Page { table, page: *page }, page.to_string()));
                }
            }
            Node::Page {
                table: Table::Object,
                page,
            } => {
                let read = self.db.begin_read().unwrap();
                let table = read.open_table(TABLE_LAYER_OBJECT).unwrap();
                for entry in table.range((page, 0)..).unwrap() {
                    let (key, _) = entry.unwrap();
                    let (p, uuid) = key.value();
                    if p != page {
                        break;
                    }
                    entries.push((Node::Object { page, uuid }, object_name(uuid)));
                }
            }
            Node::Page {
                table: Table::Chunk,
                page,
            } => {
                let read = self.db.begin_read().unwrap();
                let table = read.open_table(TABLE_LAYER_CHUNK).unwrap();
                let start = (page, (i32::MIN, i32::MIN, u8::MIN));
                for entry in table.range(start..).unwrap() {
                    let (key, _) = entry.unwrap();
                    let (p, (x, y, z)) = key.value();
                    if p != page {
                        break;
                    }
                    entries.push((Node::Chunk { page, x, y, z }, chunk_name(x, y, z)));
                }
            }
            Node::Page {
                table: Table::Raw,
                page,
            } => {
                let read = self.db.begin_read().unwrap();
                let table = read.open_table(TABLE_LAYER_CHUNK_RAW).unwrap();
                let start = (page, (i32::MIN, i32::MIN));
                for entry in table.range(start..).unwrap() {
                    let (key, _) = entry.unwrap();
                    let (p, (x, y)) = key.value();
                    if p != page {
                        break;
                    }
                    entries.push((Node::Raw { page, x, y }, raw_name(x, y)));
                }
            }
            _ => return None,
        }
        Some(entries)
    }

    fn attr(&self, req: &Request, ino: INodeNo, node: Node) -> FileAttr {
        let kind = node.kind();
        let size = match node {
            Node::Object { page, uuid } => self.object(page, uuid).map_or(0, |it| it.len() as u64),
            _ => 0,
        };
        FileAttr {
            ino,
            size,
            blocks: size.div_ceil(512),
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind,
            perm: match kind {
                FileType::Directory => 0o755,
                _ => 0o644,
            },
            nlink: match kind {
                FileType::Directory => 2,
                _ => 1,
            },
            uid: req.uid().unwrap(),
            gid: req.gid().unwrap(),
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }
}

fn object_name(uuid: u128) -> String {
    format!("{}.json", Uuid::from_u128(uuid))
}

fn object_uuid(name: &str) -> Option<u128> {
    Some(Uuid::parse_str(name.strip_suffix(".json")?).ok()?.as_u128())
}

fn chunk_name(x: i32, y: i32, z: u8) -> String {
    format!("{x}_{y}_{z}")
}

fn chunk_key(name: &str) -> Option<(i32, i32, u8)> {
    let mut parts = name.split('_');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let z = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((x, y, z))
}

fn raw_name(x: i32, y: i32) -> String {
    format!("{x}_{y}")
}

fn raw_key(name: &str) -> Option<(i32, i32)> {
    let mut parts = name.split('_');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((x, y))
}

impl Filesystem for Fuse {
    fn lookup(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(child) = name.to_str().and_then(|name| self.child(parent, name)) else {
            return reply.error(Errno::ENOENT);
        };
        let ino = self.state.lock().unwrap().intern(child);
        reply.entry(&TTL, &self.attr(req, ino, child), Generation(0));
    }

    fn getattr(&self, req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let Some(node) = self.node(ino) else {
            return reply.error(Errno::ENOENT);
        };
        reply.attr(&TTL, &self.attr(req, ino, node));
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let Some(node) = self.node(ino) else {
            return reply.error(Errno::ENOENT);
        };
        let Some(entries) = self.entries(node) else {
            return reply.error(Errno::ENOTDIR);
        };

        let mut state = self.state.lock().unwrap();
        let mut rows = vec![
            (ino, FileType::Directory, ".".to_string()),
            (
                state.intern(node.parent().unwrap_or(Node::Root)),
                FileType::Directory,
                "..".to_string(),
            ),
        ];
        for (child, name) in entries {
            rows.push((state.intern(child), child.kind(), name));
        }
        drop(state);

        for (i, (ino, kind, name)) in rows.into_iter().enumerate().skip(offset as usize) {
            if reply.add(ino, (i + 1) as u64, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn open(
        &self,
        _req: &Request,
        ino: INodeNo,
        _flags: OpenFlags,
        _kill_suid_gid: bool,
        reply: ReplyOpen,
    ) {
        if self.node(ino).is_none() {
            return reply.error(Errno::ENOENT);
        }
        reply.opened(FileHandle(0), FopenFlags::empty());
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let Some(node) = self.node(ino) else {
            return reply.error(Errno::ENOENT);
        };
        let bytes = match node {
            Node::Object { page, uuid } => self.object(page, uuid).unwrap_or_default(),
            _ => Vec::new(),
        };
        let start = (offset as usize).min(bytes.len());
        let end = (start + size as usize).min(bytes.len());
        reply.data(&bytes[start..end]);
    }

    fn write(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        let Some(node) = self.node(ino) else {
            return reply.error(Errno::ENOENT);
        };
        let Node::Object { page, uuid } = node else {
            return reply.error(Errno::EACCES);
        };

        let mut bytes = self.object(page, uuid).unwrap_or_default();
        let offset = offset as usize;
        let end = offset + data.len();
        if bytes.len() < end {
            bytes.resize(end, 0);
        }
        bytes[offset..end].copy_from_slice(data);
        self.set_object(page, uuid, &bytes);
        reply.written(data.len() as u32);
    }

    fn setattr(
        &self,
        req: &Request,
        ino: INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<FileHandle>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<fuser::BsdFileFlags>,
        _kill_suid_gid: bool,
        reply: ReplyAttr,
    ) {
        let Some(node) = self.node(ino) else {
            return reply.error(Errno::ENOENT);
        };
        if let (Some(size), Node::Object { page, uuid }) = (size, node) {
            if size == 0 {
                self.remove_object(page, uuid);
            } else if let Some(mut bytes) = self.object(page, uuid) {
                bytes.resize(size as usize, 0);
                self.set_object(page, uuid, &bytes);
            }
        }
        reply.attr(&TTL, &self.attr(req, ino, node));
    }

    fn flush(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        _lock_owner: LockOwner,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(Node::Page {
            table: Table::Object,
            page,
        }) = self.node(parent)
        else {
            return reply.error(Errno::EPERM);
        };
        let Some(uuid) = name.to_str().and_then(object_uuid) else {
            return reply.error(Errno::ENOENT);
        };
        if !self.remove_object(page, uuid) {
            return reply.error(Errno::ENOENT);
        }
        reply.ok();
    }
}

pub fn mount(db_path: &Path, mount_point: &Path) -> io::Result<()> {
    let db = Arc::new(Database::open(db_path).map_err(io::Error::other)?);
    let mut config = Config::default();
    config.mount_options = vec![MountOption::FSName("lndb".to_string())];
    fuser::mount(Fuse::new(db), mount_point, &config)
}
