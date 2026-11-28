pub mod chunk;
pub mod object;

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use ln_world::{Element, Handle, World};
use redb::{Database, WriteTransaction, backends::InMemoryBackend};
use toml_edit::{DocumentMut, Item, Value};
use winit::event::WindowEvent;

#[cfg(target_os = "android")]
use crate::lnwin::LnAndroid;
use crate::{
    lnwin::{Lnwindow, Task},
    paint::{
        input::{OffsetCorrection, TiltCorrection},
        wrapper::PageContainerArchive,
    },
    save::{
        chunk::{TABLE_LAYER_CHUNK, TABLE_LAYER_CHUNK_RAW},
        object::TABLE_LAYER_OBJECT,
    },
    ui::theme::ThemeScheme,
};

/// The format version of the database.
const DATABASE_VERSION: u32 = 0;

/// The number of backup files.
const BACKUP_SLOT: u32 = 3;

/// The minimum duration before creating another save backup
const BACKUP_MINIMUM_DURATION: Duration = Duration::from_hours(72);

/// The core database.
pub struct Save {
    sn: SceneFile,
    db: Arc<Database>,
    path: Option<SavePaths>,
}

/// None if use temp data
struct SavePaths {
    db: PathBuf,
    sn: PathBuf,
    sn_tmp: PathBuf,
}

impl Save {
    pub fn from_world(world: &World) -> Self {
        let tmp = std::env::var_os("LNPAINT_TMP_SAVE").is_some();

        if tmp {
            log::warn!("temporary save enabled by LNPAINT_TMP_SAVE");

            let mut db = Database::builder()
                .create_with_backend(InMemoryBackend::new())
                .unwrap();
            log::debug!("in memory database created");

            let mut sn_doc = SceneDocument::default();
            let sn = SceneFile::parse(&sn_doc);
            Save::touch(&sn, &mut sn_doc, &mut db, None).unwrap();
            let db = Arc::new(db);
            let path = None;
            Save { sn, db, path }
        } else {
            let path = Save::get_paths(world);
            std::fs::create_dir_all(&path.db.parent().unwrap()).unwrap();
            Save::create_backup(Some(&path.db), "old", true, BACKUP_SLOT);

            if let Ok(mut db) = Database::open(&path.db) {
                log::debug!("database loaded");

                let mut sn_doc = SceneDocument::read(&path.sn);
                let sn = SceneFile::parse(&sn_doc);
                Save::touch(&sn, &mut sn_doc, &mut db, Some(&path.db)).unwrap();
                sn_doc.write(&path.sn, &path.sn_tmp);

                let db = Arc::new(db);
                let path = Some(path);
                Save { sn, db, path }
            } else {
                let mut db = Database::create(&path.db).unwrap();
                log::debug!("database created");

                let mut sn_doc = SceneDocument::read(&path.sn);
                let sn = SceneFile::parse(&sn_doc);
                Save::touch(&sn, &mut sn_doc, &mut db, Some(&path.db)).unwrap();
                sn_doc.write(&path.sn, &path.sn_tmp);
                let db = Arc::new(db);
                let path = Some(path);
                Save { sn, db, path }
            }
        }
    }

    pub fn sn(&self) -> &SceneFile {
        &self.sn
    }

    pub fn sn_read(&self) -> SceneDocument {
        if let Some(path) = &self.path {
            SceneDocument::read(&path.sn)
        } else {
            SceneDocument::default()
        }
    }

    pub fn sn_write(&mut self, sn_doc: &SceneDocument) {
        if let Some(path) = &self.path {
            sn_doc.write(&path.sn, &path.sn_tmp);
            self.sn = SceneFile::parse(sn_doc);
        }
    }

    pub fn db(&self) -> Arc<Database> {
        self.db.clone()
    }

    fn get_paths(world: &World) -> SavePaths {
        SavePaths {
            db: get_file_path(world, "world.lndb"),
            sn: get_file_path(world, "world.toml"),
            sn_tmp: get_file_path(world, "world.toml~"),
        }
    }

    /// Touch a existed database, including updating necessary timestamps,
    /// validation, and most of all migration data from older versions.
    fn touch(
        sn: &SceneFile,
        sn_doc: &mut SceneDocument,
        db: &mut Database,
        db_file: Option<&Path>,
    ) -> Result<(), redb::Error> {
        if let Some(version) = sn.database_version {
            if version > DATABASE_VERSION {
                panic!("cannot open database from newer version {version}",);
            } else if version < DATABASE_VERSION {
                Self::create_backup(db_file, "migration", false, 256);
                Self::migrate_format(db, version)?;
                sn_doc.set_database_version(DATABASE_VERSION);
            }
        } else {
            sn_doc.set_database_version(DATABASE_VERSION);
        }

        if chunk::legacy_classic::need_migration(db)? {
            Self::create_backup(db_file, "classic", false, 4);
            chunk::legacy_classic::startup_migration(db)?;
        }

        {
            let mut write = db.begin_write()?;
            write.set_quick_repair(true);
            write.open_table(TABLE_LAYER_CHUNK)?;
            write.open_table(TABLE_LAYER_CHUNK_RAW)?;
            write.open_table(TABLE_LAYER_OBJECT)?;
            write.commit()?;
        }

        if sn.compact_on_startup {
            Self::perform_compact(db)?;
            sn_doc.set_compact_on_startup(false);
        }

        Ok(())
    }

    fn migrate_format(db: &Database, version: u32) -> Result<(), redb::Error> {
        log::info!("start migration from {version} to {DATABASE_VERSION}");

        let write = db.begin_write()?;
        #[expect(unreachable_code)]
        for migrate_format in version..DATABASE_VERSION {
            match migrate_format {
                _ => unimplemented!("unsupported migration {migrate_format}"),
            }

            log::info!("finish migration from {migrate_format}");
        }
        write.commit()?;

        log::info!("migration all finished");
        Ok(())
    }

    fn perform_compact(db: &mut Database) -> Result<(), redb::Error> {
        log::debug!("database compact started");
        let result = db.compact()?;
        log::debug!("database compact finished, result: {result}");

        Ok(())
    }

    fn create_backup(file: Option<&Path>, key: &'static str, skippable: bool, slot: u32) {
        let Some(file) = file else {
            return;
        };

        let Ok(true) = std::fs::exists(file) else {
            return;
        };

        let mut temp = PathBuf::new();
        let mut backup = PathBuf::new();
        let mut oldest_backup = PathBuf::new();
        let mut newest_backup = PathBuf::new();
        let mut oldest = None;
        let mut newest = None;
        for i in 0..slot {
            temp.clear();
            temp.push(file);
            temp.add_extension(&i.to_string());
            temp.add_extension(key);

            let Ok(metadata) = std::fs::metadata(&temp) else {
                log::debug!("backup slot {temp:?} is empty");
                backup.clone_from(&temp);
                break;
            };

            let Ok(modified) = metadata.modified() else {
                log::debug!("cannot reach metadata of {temp:?}");
                backup.clone_from(&temp);
                break;
            };

            let duration = SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default();

            if oldest.is_none_or(|it| duration > it) {
                backup.clone_from(&temp);
                oldest_backup.clone_from(&temp);
                oldest = Some(duration);
            }

            if newest.is_none_or(|it| duration < it) {
                newest_backup.clone_from(&temp);
                newest = Some(duration);
            }
        }

        log::debug!("newest {newest_backup:?} {newest:?}");
        log::debug!("oldest {oldest_backup:?} {oldest:?}");

        if skippable && newest.is_some_and(|it| it < BACKUP_MINIMUM_DURATION) {
            log::debug!("backup skipped: very recent backup file");
            return;
        }

        log::debug!("backup file is written to {backup:?}");
        std::fs::copy(file, backup).unwrap();
    }
}

impl Element for Save {}

// Scene File //

/// read-only deserialized copy from origin scene file
#[derive(Default)]
pub struct SceneFile {
    pub database_version: Option<u32>,
    pub compact_on_startup: bool,
    pub desktop_mode: bool,
    pub theme: ThemeArchive,
    pub page_camera: PageContainerArchive,
    pub tilt_correction: Option<TiltCorrection>,
    pub offset_correction: Option<OffsetCorrection>,
}

#[derive(Default)]
pub struct ThemeArchive {
    pub scheme: Option<ThemeScheme>,
    pub hue: Option<f32>,
}

impl SceneFile {
    /// Parsing is hand-written since user's configuration file should be treated as untrusted, while
    /// attempting to make default Deserialize derivation work is too complex for me.
    fn parse(sn_doc: &SceneDocument) -> Self {
        let t = sn_doc.doc.as_table();
        SceneFile {
            database_version: t
                .get("database_version")
                .and_then(|x| x.as_integer())
                .map(|x| x as u32),
            compact_on_startup: t
                .get("compact_on_startup")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            desktop_mode: t
                .get("desktop_mode")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            theme: ThemeArchive {
                scheme: t
                    .get("theme")
                    .and_then(|x| x.get("scheme"))
                    .and_then(deserialize_item)
                    .and_then(|x: String| match x.as_str() {
                        "dark" => Some(ThemeScheme::Dark),
                        "light" => Some(ThemeScheme::Light),
                        "dark_variant" => Some(ThemeScheme::DarkVariant),
                        "light_variant" => Some(ThemeScheme::LightVariant),
                        "neutral" => Some(ThemeScheme::Neutral),
                        "neutral_variant" => Some(ThemeScheme::NeutralVariant),
                        unknown => {
                            log::warn!("unknown color scheme: {unknown}");
                            None
                        }
                    }),
                hue: t
                    .get("theme")
                    .and_then(|x| x.get("hue"))
                    .and_then(deserialize_item),
            },
            page_camera: t
                .get("page_camera")
                .and_then(deserialize_item)
                .unwrap_or_else(|| {
                    log::warn!("use default page_camera");
                    Default::default()
                }),
            tilt_correction: t
                .get("tilt_correction")
                .and_then(deserialize_item)
                .unwrap_or_default(),
            offset_correction: t
                .get("offset_correction")
                .and_then(deserialize_item)
                .unwrap_or_default(),
        }
    }
}

/// I don't know why or how they make deserializing so hard to use, fking non-sense at all.
///
/// `toml_edit` support to use [`ValueDeserializer`](toml_edit::de::ValueDeserializer) to deserialize
/// values, but which is inaccessible through simpler API other than using `IntoDeserializer`. Why
/// there is anyone think it is a good idea to use implicit, anonymous `Into`-ish style API?
fn deserialize_item<'de, T: serde::Deserialize<'de>>(item: &Item) -> Option<T> {
    let value = match item.clone().into_value() {
        Ok(x) => x,
        // Item::None
        Err(_) => return None,
    };
    let deserializer = serde::de::IntoDeserializer::into_deserializer(value);
    match <T as serde::Deserialize>::deserialize(deserializer) {
        Ok(x) => Some(x),
        Err(err) => {
            log::warn!("failed deserializing toml item:\n {err}");
            None
        }
    }
}

// Scene Document //

#[derive(Debug, Default)]
pub struct SceneDocument {
    doc: DocumentMut,
    // false if document is from a malformed parsing process
    safe: bool,
}

impl SceneDocument {
    fn read(sn_file: &Path) -> Self {
        match std::fs::read_to_string(&sn_file) {
            Ok(text) => match text.parse::<DocumentMut>() {
                Ok(doc) => SceneDocument { doc, safe: true },
                Err(err) => {
                    log::warn!("scene toml parse error: {err}, use empty toml to continue");
                    SceneDocument::default()
                }
            },
            Err(err) => match err.kind() {
                ErrorKind::NotFound => SceneDocument {
                    doc: DocumentMut::new(),
                    safe: true,
                },
                err => {
                    log::warn!("scene read failed: {err}, use empty toml to continue");
                    SceneDocument::default()
                }
            },
        }
    }

    fn write(&self, sn_file: &Path, sn_tmp_file: &Path) {
        if !self.safe {
            // malformed, don't corrupt user's data
            return;
        }

        std::fs::write(&sn_tmp_file, self.doc.to_string()).unwrap();
        std::fs::rename(&sn_tmp_file, &sn_file).unwrap();
    }

    pub fn set_database_version(&mut self, version: u32) {
        self.doc["database_version"] = toml_edit::value(version as i64);
    }

    pub fn set_compact_on_startup(&mut self, compact: bool) {
        self.doc["compact_on_startup"] = toml_edit::value(compact);
    }

    pub fn set_page_camera(&mut self, camera: PageContainerArchive) {
        let camera = toml_edit::ser::to_document(&camera).unwrap();
        self.doc["page_camera"] = match camera.into_item() {
            Item::Table(t) => Item::Table(t),
            Item::Value(Value::InlineTable(t)) => Item::Table(t.into_table()),
            it => it,
        };
    }
}
pub struct Autosave(pub Box<dyn FnMut(&World, AutosaveTask) + Send>);

pub struct AutosaveTask<'a> {
    pub write: &'a WriteTransaction,
    pub sn: &'a SceneFile,
    pub sn_doc: &'a mut SceneDocument,
}

pub struct AutosaveScheduler {
    pub autosave_duration: Duration,
}

impl Autosave {
    pub fn autosave_all(world: &World) {
        let start = Instant::now();

        let mut save = world.single_fetch_mut::<Save>();
        let mut sn_doc = save.sn_read();
        let mut write = save.db.begin_write().unwrap();
        write.set_quick_repair(true);
        world.foreach_fetch_mut::<Autosave>(|mut task| {
            (task.0)(
                world,
                AutosaveTask {
                    write: &write,
                    sn: &save.sn,
                    sn_doc: &mut sn_doc,
                },
            );
        });
        save.sn_write(&sn_doc);
        write.commit().unwrap();

        let duration = Instant::now().duration_since(start);
        log::debug!("autosave request finished in {duration:?}");
    }
}

impl Element for Autosave {}

fn autosave_task(world: &World, duration: Duration) {
    world.insert(Task {
        instant: Instant::now() + duration,
        content: Box::new(move |world| {
            Autosave::autosave_all(world);
            autosave_task(world, duration);
        }),
    });
}

impl Element for AutosaveScheduler {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("autosave_scheduler");
        world.set_autodeps(Some(this));

        autosave_task(world, self.autosave_duration);

        world.observer(
            world.single::<Lnwindow>(),
            |event: &WindowEvent, world| match event {
                WindowEvent::CloseRequested => Autosave::autosave_all(world),
                #[cfg(target_os = "android")]
                WindowEvent::Focused(false) => Autosave::autosave_all(world),
                _ => (),
            },
        );
    }
}

#[cfg(target_os = "android")]
pub fn get_file_path(world: &World, filename: &str) -> PathBuf {
    let app = world.single_fetch::<LnAndroid>();
    let mut path = app.0.external_data_path().unwrap();
    path.push(filename);
    path
}

#[cfg(not(target_os = "android"))]
pub fn get_file_path(_world: &World, filename: &str) -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap();
    match option_env!("LNPAINT_RELEASE").is_some() {
        true => path.push("LnPaint"),
        false => path.push("LnPaintDev"),
    }
    path.push(filename);
    path
}
