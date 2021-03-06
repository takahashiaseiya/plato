extern crate libc;

use std::ptr;
use std::mem;
use std::rc::Rc;
use std::path::Path;
use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use document::{Document, TextLayer, LayerGrain, TocEntry, Link};
use framebuffer::Pixmap;
use geom::Rectangle;

const DDJVU_JOB_OK: JobStatus = 2;
const DDJVU_JOB_FAILED: JobStatus = 3;

const DDJVU_ERROR: MessageTag = 0;
const DDJVU_INFO: MessageTag = 1;
const DDJVU_NEWSTREAM: MessageTag = 2;
const DDJVU_DOCINFO: MessageTag = 3;
const DDJVU_PAGEINFO: MessageTag = 4;
const DDJVU_RELAYOUT: MessageTag = 5;
const DDJVU_REDISPLAY: MessageTag = 6;
const DDJVU_CHUNK: MessageTag = 7;
const DDJVU_THUMBNAIL: MessageTag = 8;
const DDJVU_PROGRESS: MessageTag = 9;

const DDJVU_FORMAT_BGR24: FormatStyle = 0;
const DDJVU_FORMAT_RGB24: FormatStyle = 1;
const DDJVU_FORMAT_RGBMASK16: FormatStyle = 2;
const DDJVU_FORMAT_RGBMASK32: FormatStyle = 3;
const DDJVU_FORMAT_GREY8: FormatStyle = 4;

const DDJVU_RENDER_COLOR: RenderMode = 0;

const MINIEXP_NIL: *mut MiniExp = 0 as *mut MiniExp;
const MINIEXP_DUMMY: *mut MiniExp = 2 as *mut MiniExp;

const CACHE_SIZE: libc::c_ulong = 32 * 1024 * 1024;

enum ExoContext {}
enum ExoDocument {}
enum ExoFormat {}
enum ExoJob {}
enum ExoPage {}
enum MiniExp {}

type JobStatus = libc::c_uint;
type MessageTag = libc::c_uint;
type RenderMode = libc::c_uint;
type FormatStyle = libc::c_uint;

#[link(name="djvulibre")]
extern {
    fn ddjvu_context_create(name: *const libc::c_char) -> *mut ExoContext;
    fn ddjvu_context_release(ctx: *mut ExoContext);
    fn ddjvu_cache_set_size(ctx: *mut ExoContext, size: libc::c_ulong);
    fn ddjvu_cache_clear(ctx: *mut ExoContext);
    fn ddjvu_message_wait(ctx: *mut ExoContext) -> *mut Message;
    fn ddjvu_message_pop(ctx: *mut ExoContext);
    fn ddjvu_document_job(doc: *mut ExoDocument) -> *mut ExoJob;
    fn ddjvu_page_job(page: *mut ExoPage) -> *mut ExoJob;
    fn ddjvu_job_status(job: *mut ExoJob) -> JobStatus;
    fn ddjvu_job_release(job: *mut ExoJob);
    fn ddjvu_document_create_by_filename_utf8(ctx: *mut ExoContext, path: *const libc::c_char, cache: libc::c_int) -> *mut ExoDocument;
    fn ddjvu_document_get_pagenum(doc: *mut ExoDocument) -> libc::c_int;
    fn ddjvu_page_create_by_pageno(doc: *mut ExoDocument, page_idx: libc::c_int) -> *mut ExoPage;
    fn ddjvu_page_create_by_pageid(doc: *mut ExoDocument, pageid: *const libc::c_char) -> *mut ExoPage;
    fn ddjvu_page_get_width(page: *mut ExoPage) -> libc::c_int;
    fn ddjvu_page_get_height(page: *mut ExoPage) -> libc::c_int;
    fn ddjvu_page_get_resolution(page: *mut ExoPage) -> libc::c_int;
    fn ddjvu_page_get_rotation(page: *mut ExoPage) -> libc::c_uint;
    fn ddjvu_page_render(page: *mut ExoPage, mode: RenderMode, p_rect: *const DjvuRect, r_rect: *const DjvuRect, fmt: *const ExoFormat, row_size: libc::c_ulong, buf: *mut u8) -> libc::c_int;
    fn ddjvu_format_create(style: FormatStyle, nargs: libc::c_int, args: *const libc::c_uint) -> *mut ExoFormat;
    fn ddjvu_format_release(fmt: *mut ExoFormat);
    fn ddjvu_format_set_row_order(fmt: *mut ExoFormat, top_to_bottom: libc::c_int);
    fn ddjvu_format_set_y_direction(fmt: *mut ExoFormat, top_to_bottom: libc::c_int);
    fn ddjvu_document_get_pagetext(doc: *mut ExoDocument, page_idx: libc::c_int, max_detail: *const libc::c_char) -> *mut MiniExp;
    fn ddjvu_document_get_outline(doc: *mut ExoDocument) -> *mut MiniExp;
    fn ddjvu_document_get_anno(doc: *mut ExoDocument, compat: libc::c_int) -> *mut MiniExp;
    fn ddjvu_document_get_pageanno(doc: *mut ExoDocument, page_idx: libc::c_int) -> *mut MiniExp;
    fn ddjvu_anno_get_hyperlinks(annot: *mut MiniExp) -> *mut *mut MiniExp;
    fn ddjvu_anno_get_metadata_keys(annot: *mut MiniExp) -> *mut *mut MiniExp;
    fn ddjvu_anno_get_metadata(annot: *mut MiniExp, key: *const MiniExp) -> *const libc::c_char;
    fn ddjvu_miniexp_release(document: *mut ExoDocument, exp: *mut MiniExp);
    fn miniexp_symbol(s: *const libc::c_char) -> *const MiniExp;
    fn miniexp_length(exp: *mut MiniExp) -> libc::c_int;
    fn miniexp_nth(n: libc::c_int, list: *mut MiniExp) -> *mut MiniExp;
    fn miniexp_stringp(exp: *mut MiniExp) -> libc::c_int;
    fn miniexp_to_str(exp: *mut MiniExp) -> *const libc::c_char;
    fn miniexp_to_name(sym: *mut MiniExp) -> *const libc::c_char;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DjvuRect {
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub w: libc::c_uint,
    pub h: libc::c_uint,
}

impl Into<DjvuRect> for Rectangle {
    fn into(self) -> DjvuRect {
        DjvuRect {
            x: self.min.y as libc::c_int,
            y: self.min.x as libc::c_int,
            w: self.width() as libc::c_uint,
            h: self.height() as libc::c_uint,
        }
    }
}

impl Default for DjvuRect {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[repr(C)]
struct Message {
    tag: MessageTag,
    context: *mut ExoContext,
    document: *mut ExoDocument,
    page: *mut ExoPage,
    job: *mut ExoJob,
    u: MessageBlob,
}

#[repr(C)]
union MessageBlob {
    error: MessageError,
    info: MessageInfo,
    new_stream: MessageNewStream,
    chunk: MessageChunk,
    thumbnail: MessageThumbnail,
    progress: MessageProgress,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MessageError {
    message: *const libc::c_char,
    function: *const libc::c_char,
    filename: *const libc::c_char,
    lineno: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MessageInfo {
    message: *const libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MessageNewStream {
    streamid: libc::c_int,
    name: *const libc::c_char,
    url: *const libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MessageChunk {
    chunkid: *const libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MessageThumbnail {
    pagenum: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MessageProgress {
    status: JobStatus,
    percent: libc::c_int,
}

struct DjvuContext(*mut ExoContext);

pub struct DjvuOpener(Rc<DjvuContext>);

pub struct DjvuDocument {
    ctx: Rc<DjvuContext>,
    doc: *mut ExoDocument,
}

pub struct DjvuPage<'a> {
    page: *mut ExoPage,
    doc: &'a DjvuDocument,
}

impl DjvuContext {
    fn handle_message(&self) {
        unsafe {
            let msg = ddjvu_message_wait(self.0);
            match (*msg).tag {
                DDJVU_ERROR => {
                    let msg = (*msg).u.error;
                    let message = CStr::from_ptr(msg.message).to_string_lossy();
                    let filename = msg.filename;
                    let lineno = msg.lineno;
                    if filename.is_null() {
                        eprintln!("Error: {}.", message);
                    } else {
                        let filename = CStr::from_ptr(filename).to_string_lossy();
                        eprintln!("Error: {}: '{}:{}'.", message, filename, lineno);
                    }
                },
                _ => (),
            }
            ddjvu_message_pop(self.0);
        }
    }
}

impl LayerGrain {
    pub fn from_bytes(name: &[u8]) -> LayerGrain {
        match name {
            b"page" => LayerGrain::Page,
            b"column" => LayerGrain::Column,
            b"region" => LayerGrain::Region,
            b"para" => LayerGrain::Paragraph,
            b"line" => LayerGrain::Line,
            b"word" => LayerGrain::Word,
            b"char" => LayerGrain::Character,
            _ => LayerGrain::Character,
        }
    }
}

impl DjvuOpener {
    pub fn new() -> Option<DjvuOpener> {
        unsafe {
            let name = CString::new("Plato").unwrap();
            let ctx = ddjvu_context_create(name.as_ptr());
            if ctx.is_null() {
                None
            } else {
                ddjvu_cache_set_size(ctx, CACHE_SIZE);
                Some(DjvuOpener(Rc::new(DjvuContext(ctx))))
            }
        }
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> Option<DjvuDocument> {
        unsafe {
            let c_path = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
            let doc = ddjvu_document_create_by_filename_utf8((self.0).0,
                                                             c_path.as_ptr(),
                                                             1);
            if doc.is_null() {
                return None;
            }
            let job = ddjvu_document_job(doc);
            while ddjvu_job_status(job) < DDJVU_JOB_OK {
                self.0.handle_message();
            }
            if ddjvu_job_status(job) >= DDJVU_JOB_FAILED {
                None
            } else {
                Some(DjvuDocument {
                    ctx: self.0.clone(),
                    doc: doc,
                })
            }
        }
    }
}

impl Document for DjvuDocument {
    fn pages_count(&self) -> usize {
        unsafe { ddjvu_document_get_pagenum(self.doc) as usize }
    }

    fn pixmap(&self, index: usize, scale: f32) -> Option<Pixmap> {
        self.page(index).and_then(|p| p.pixmap(scale))
    }

    fn dims(&self, index: usize) -> Option<(f32, f32)> {
        self.page(index).map(|page| {
            let dims = page.dims();
            (dims.0 as f32, dims.1 as f32)
        })
    }

    fn toc(&self) -> Option<Vec<TocEntry>> {
        unsafe {
            let mut exp = ddjvu_document_get_outline(self.doc);
            while exp == MINIEXP_DUMMY {
                self.ctx.handle_message();
                exp = ddjvu_document_get_outline(self.doc);
            }
            if exp == MINIEXP_NIL {
                None
            } else {
                let toc = Self::walk_toc(exp);
                ddjvu_miniexp_release(self.doc, exp);
                Some(toc)
            }
        }
    }

    fn text(&self, index: usize) -> Option<TextLayer> {
        unsafe {
            let page = self.page(index);
            if page.is_none() {
                return None;
            }
            let height = page.unwrap().height() as i32;
            let grain = CString::new("word").unwrap();
            let mut exp = ddjvu_document_get_pagetext(self.doc, index as libc::c_int, grain.as_ptr());
            while exp == MINIEXP_DUMMY {
                self.ctx.handle_message();
                exp = ddjvu_document_get_pagetext(self.doc, index as libc::c_int, grain.as_ptr());
            }
            if exp == MINIEXP_NIL {
                None
            } else {
                let text_page = Self::walk_text(exp, height);
                ddjvu_miniexp_release(self.doc, exp);
                Some(text_page)
            }
        }
    }

    fn links(&self, index: usize) -> Option<Vec<Link>> {
        unsafe {
            let mut exp = ddjvu_document_get_pageanno(self.doc, index as libc::c_int);
            while exp == MINIEXP_DUMMY {
                self.ctx.handle_message();
                exp = ddjvu_document_get_pageanno(self.doc, index as libc::c_int);
            }
            if exp == MINIEXP_NIL {
                None
            } else {
                let links = ddjvu_anno_get_hyperlinks(exp);
                if links.is_null() {
                    ddjvu_miniexp_release(self.doc, exp);
                    return None;
                }
                let height = self.page(index).map(|p| p.height()).unwrap() as i32;
                let c_rect = CString::new("rect").unwrap();
                let s_rect = miniexp_symbol(c_rect.as_ptr()) as *mut MiniExp;
                let mut link = links;
                let mut result = Vec::new();
                while !(*link).is_null() {
                    let uri = miniexp_nth(1, *link);
                    let area = miniexp_nth(3, *link);
                    if miniexp_stringp(uri) == 1 && miniexp_nth(0, area) == s_rect {
                        let uri = CStr::from_ptr(miniexp_to_str(uri)).to_string_lossy().into_owned();
                        let rect = {
                            let x_min = miniexp_nth(1, area) as i32 >> 2;
                            let y_max = height - (miniexp_nth(2, area) as i32 >> 2);
                            let r_width = miniexp_nth(3, area) as i32 >> 2;
                            let r_height = miniexp_nth(4, area) as i32 >> 2;
                            rect![x_min, y_max - r_height, x_min + r_width, y_max]
                        };
                        result.push(Link { uri, rect });
                    }
                    link = link.offset(1);
                }
                libc::free(links as *mut libc::c_void);
                ddjvu_miniexp_release(self.doc, exp);
                Some(result)
            }
        }
    }

    fn title(&self) -> Option<String> {
        self.info("title")
    }

    fn author(&self) -> Option<String> {
        self.info("author")
    }

    fn is_reflowable(&self) -> bool {
        false
    }

    fn layout(&mut self, _width: f32, _height: f32, _em: f32) {
    }
}

impl DjvuDocument {
    pub fn page(&self, index: usize) -> Option<DjvuPage> {
        unsafe {
            let page = ddjvu_page_create_by_pageno(self.doc, index as libc::c_int);
            if page.is_null() {
                return None;
            }
            let job = ddjvu_page_job(page);
            while ddjvu_job_status(job) < DDJVU_JOB_OK {
                self.ctx.handle_message();
            }
            if ddjvu_job_status(job) >= DDJVU_JOB_FAILED {
                None
            } else {
                Some(DjvuPage { page, doc: self })
            }
        }
    }

    fn walk_text(exp: *mut MiniExp, height: i32) -> TextLayer {
        unsafe {
            let len = miniexp_length(exp);
            let mut text: Option<String> = None;
            let p_rect = {
                let x_min = miniexp_nth(1, exp) as i32 >> 2;
                let y_max = height - (miniexp_nth(2, exp) as i32 >> 2);
                let x_max = miniexp_nth(3, exp) as i32 >> 2;
                let y_min = height - (miniexp_nth(4, exp) as i32 >> 2);
                rect![x_min, y_min, x_max, y_max]
            };
            let grain = {
                let raw = miniexp_to_name(miniexp_nth(0, exp));
                let c_str = CStr::from_ptr(raw);
                LayerGrain::from_bytes(c_str.to_bytes())
            };
            let mut children = Vec::new();
            if miniexp_stringp(miniexp_nth(5, exp)) == 1 {
                let raw = miniexp_to_str(miniexp_nth(5, exp));
                let c_str = CStr::from_ptr(raw);
                text = Some(c_str.to_string_lossy().into_owned());
            } else {
                for i in 5..len {
                    let child = miniexp_nth(i, exp);
                    children.push(Self::walk_text(child, height));
                }
            }
            TextLayer {
                grain: grain,
                rect: p_rect,
                text: text,
                children: children,
            }
        }
    }

    fn walk_toc(exp: *mut MiniExp) -> Vec<TocEntry> {
        unsafe {
            let mut vec = Vec::new();
            let len = miniexp_length(exp);
            for i in 0..len {
                let itm = miniexp_nth(i, exp);
                // Skip `itm` if it isn't a list.
                if (itm as libc::size_t) & 3 != 0 {
                    continue;
                }
                let raw = miniexp_to_str(miniexp_nth(0, itm));
                let title = CStr::from_ptr(raw).to_string_lossy().into_owned();
                let raw = miniexp_to_str(miniexp_nth(1, itm));
                let bytes = CStr::from_ptr(raw).to_bytes();
                // TODO: handle the case #page_name: we need to call ddjvu_document_get_fileinfo
                // for every file and try to find a matching page_name
                let digits = bytes.iter().map(|v| *v as u8 as char)
                                         .filter(|c| c.is_digit(10))
                                         .collect::<String>();
                let page = digits.parse::<usize>().unwrap_or(1).saturating_sub(1);
                let children = if miniexp_length(itm) > 2 {
                    Self::walk_toc(itm)
                } else {
                    Vec::new()
                };
                vec.push(TocEntry {
                    title: title,
                    page: page,
                    children: children,
                });
            }
            vec
        }
    }

    pub fn info(&self, key: &str) -> Option<String> {
        unsafe {
            let mut exp = ddjvu_document_get_anno(self.doc, 1);
            while exp == MINIEXP_DUMMY {
                self.ctx.handle_message();
                exp = ddjvu_document_get_anno(self.doc, 1);
            }
            if exp == MINIEXP_NIL {
                None
            } else {
                let key = CString::new(key).unwrap();
                let key = miniexp_symbol(key.as_ptr());
                let val = ddjvu_anno_get_metadata(exp, key);
                if val.is_null() {
                    None
                } else {
                    ddjvu_miniexp_release(self.doc, exp);
                    Some(CStr::from_ptr(val).to_string_lossy().into_owned())
                }
            }
        }
    }
}

impl<'a> DjvuPage<'a> {
    pub fn pixmap(&self, scale: f32) -> Option<Pixmap> {
        unsafe {
            let (width, height) = self.dims();
            let rect = DjvuRect {
                x: 0,
                y: 0,
                w: (scale * width as f32) as libc::c_uint,
                h: (scale * height as f32) as libc::c_uint,
            };

            let fmt = ddjvu_format_create(DDJVU_FORMAT_GREY8, 0, ptr::null());

            if fmt.is_null() {
                return None;
            }

            ddjvu_format_set_row_order(fmt, 1);
            ddjvu_format_set_y_direction(fmt, 1);

            let len = (rect.w * rect.h) as usize;
            let mut buf = vec![0xff; len];

            ddjvu_page_render(self.page, DDJVU_RENDER_COLOR,
                              &rect, &rect, fmt,
                              rect.w as libc::c_ulong, buf.as_mut_ptr());

            let job = ddjvu_page_job(self.page);

            while ddjvu_job_status(job) < DDJVU_JOB_OK {
                self.doc.ctx.handle_message();
            }

            ddjvu_format_release(fmt);

            if ddjvu_job_status(job) >= DDJVU_JOB_FAILED {
                return None;
            }

            Some(Pixmap { width: rect.w as i32,
                          height: rect.h as i32,
                          buf })
        }
    }

    pub fn dims(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    pub fn width(&self) -> u32 {
        unsafe { ddjvu_page_get_width(self.page) as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { ddjvu_page_get_height(self.page) as u32 }
    }

    pub fn dpi(&self) -> u16 {
        unsafe { ddjvu_page_get_resolution(self.page) as u16 }
    }
}

impl<'a> Drop for DjvuPage<'a> {
    fn drop(&mut self) {
        unsafe { ddjvu_job_release(ddjvu_page_job(self.page)); }
    }
}

impl Drop for DjvuDocument {
    fn drop(&mut self) {
        unsafe { ddjvu_job_release(ddjvu_document_job(self.doc)); }
    }
}

impl Drop for DjvuContext {
    fn drop(&mut self) {
        unsafe { ddjvu_context_release(self.0); }
    }
}
