//! The looping animation on Skip Dat's install screen: the glass soap bar floating on a pink
//! liquid, bobbing gently, with glass bubbles rising and light sweeping across it. Everything is
//! periodic over 6 seconds, so the loop is seamless.
//!
//! The installer starts it with its progress page's window and the area to fill:
//!     setup-anim.exe <parent hwnd> <x> <y> <width> <height>
//! It draws with Direct2D (on the graphics card) about 60 times a second into a child window of
//! that page, and quits by itself when the page goes away.
#![windows_subsystem = "windows"]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    if let Err(e) = imp::run() {
        // nobody to tell: the installer carries on without the animation (logged when testing)
        imp::log(&format!("error: {:?} {}", e.code(), e.message()));
    }
}

#[cfg(windows)]
mod imp {
    use std::f32::consts::TAU;
    use std::time::Instant;

    use windows::core::{w, Interface, Result, PCWSTR, HSTRING};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Direct2D::Common::{
        D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED, D2D1_GRADIENT_STOP,
        D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
    };
    use windows::Win32::Graphics::Direct2D::*;
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
    use windows::Win32::Graphics::Gdi::{GetSysColor, ValidateRect, COLOR_BTNFACE};
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows_numerics::{Matrix3x2, Vector2};

    const LOOP: f32 = 6.0;

    /// Diagnostics, only when SKIPDAT_ANIM_LOG names a file (used when testing the installer).
    pub fn log(msg: &str) {
        if let Ok(p) = std::env::var("SKIPDAT_ANIM_LOG") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let _ = writeln!(f, "{msg}");
            }
        }
    }
    static mut FRAMES: u32 = 0;
    static ICON: &[u8] = include_bytes!("../../../branding/out/icon.png");

    struct Scene {
        rt: ID2D1RenderTarget,
        factory: ID2D1Factory,
        icon: ID2D1Bitmap,
        w: f32,
        h: f32,
        start: Instant,
        bg: D2D1_COLOR_F,
    }

    static mut SCENE: Option<Scene> = None;
    static mut PARENT: HWND = HWND(std::ptr::null_mut());

    fn col(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
        D2D1_COLOR_F { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a }
    }
    fn pt(x: f32, y: f32) -> Vector2 {
        Vector2 { X: x, Y: y }
    }

    /// Test mode: `--frames <folder> <w> <h> <count>` renders the loop to PNG files offscreen.
    unsafe fn frames(dir: &str, w: u32, h: u32, count: u32) -> Result<()> {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        log("frames: wic ok");
        let bmp = wic.CreateBitmap(w, h, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)?;
        log("frames: bitmap ok");
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let props = D2D1_RENDER_TARGET_PROPERTIES {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: 96.0,
            dpiY: 96.0,
            ..Default::default()
        };
        let rt = factory.CreateWicBitmapRenderTarget(&bmp, &props)?;
        log("frames: target ok");
        let icon = load_icon(&rt)?;
        log("frames: icon ok");
        let s = Scene { rt, factory, icon, w: w as f32, h: h as f32, start: Instant::now(), bg: col(240, 240, 240, 1.0) };
        for i in 0..count {
            draw_at(&s, LOOP * i as f32 / count as f32)?;
            if i == 0 { log("frames: first frame drawn"); }
            let stream = wic.CreateStream()?;
            stream.InitializeFromFilename(&HSTRING::from(format!("{dir}\\f{i:03}.png")), windows::Win32::Foundation::GENERIC_WRITE.0)?;
            let enc = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
            enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
            let mut frame = None;
            enc.CreateNewFrame(&mut frame, std::ptr::null_mut())?;
            let frame = frame.unwrap();
            frame.Initialize(None)?;
            frame.SetSize(w, h)?;
            let mut fmt = GUID_WICPixelFormat32bppPBGRA;
            frame.SetPixelFormat(&mut fmt)?;
            frame.WriteSource(&bmp, std::ptr::null())?;
            frame.Commit()?;
            enc.Commit()?;
        }
        Ok(())
    }

    pub fn run() -> Result<()> {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        if raw.first().map(|a| a == "--frames").unwrap_or(false) && raw.len() >= 5 {
            return unsafe { frames(&raw[1], raw[2].parse().unwrap_or(450), raw[3].parse().unwrap_or(165), raw[4].parse().unwrap_or(60)) };
        }
        let args: Vec<i64> = raw.iter().filter_map(|a| a.parse().ok()).collect();
        if args.len() < 5 {
            return Ok(());
        }
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            PARENT = HWND(args[0] as isize as *mut _);
            if !IsWindow(Some(PARENT)).as_bool() {
                return Ok(());
            }
            let inst = GetModuleHandleW(None)?;
            let class = w!("SkipDatSetupAnim");
            let wc = WNDCLASSW { lpfnWndProc: Some(proc_), hInstance: inst.into(), lpszClassName: class, hCursor: LoadCursorW(None, IDC_ARROW)?, ..Default::default() };
            RegisterClassW(&wc);
            let (x, y, w, h) = (args[1] as i32, args[2] as i32, args[3].max(10) as i32, args[4].max(10) as i32);
            log(&format!("parent {:?} rect {x},{y} {w}x{h}", PARENT));
            let hwnd = CreateWindowExW(Default::default(), class, PCWSTR::null(), WS_CHILD | WS_VISIBLE, x, y, w, h, Some(PARENT), None, Some(inst.into()), None);
            log(&format!("child window: {:?}", hwnd.as_ref().map(|h| h.0).map_err(|e| e.message())));
            let hwnd = hwnd?;
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            };
            let hprops = D2D1_HWND_RENDER_TARGET_PROPERTIES { hwnd, pixelSize: D2D_SIZE_U { width: w as u32, height: h as u32 }, presentOptions: D2D1_PRESENT_OPTIONS_NONE };
            let rt = factory.CreateHwndRenderTarget(&props, &hprops);
            log(&format!("render target: {}", rt.as_ref().map(|_| "ok".to_string()).unwrap_or_else(|e| e.message())));
            let rt = rt?;
            let icon = load_icon(&rt.cast()?);
            log(&format!("icon: {}", icon.as_ref().map(|_| "ok".to_string()).unwrap_or_else(|e| e.message())));
            let icon = icon?;
            let c = GetSysColor(COLOR_BTNFACE);
            let bg = col((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8, 1.0);
            SCENE = Some(Scene { rt: rt.cast()?, factory, icon, w: w as f32, h: h as f32, start: Instant::now(), bg });
            SetTimer(Some(hwnd), 1, 16, None);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        Ok(())
    }

    unsafe fn load_icon(rt: &ID2D1RenderTarget) -> Result<ID2D1Bitmap> {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = wic.CreateStream()?;
        stream.InitializeFromMemory(ICON)?;
        let dec = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)?;
        let frame = dec.GetFrame(0)?;
        let conv = wic.CreateFormatConverter()?;
        conv.Initialize(&frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeMedianCut)?;
        rt.CreateBitmapFromWicBitmap(&conv, None)
    }

    unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_TIMER => {
                // the installer page is gone: so are we
                if !IsWindow(Some(PARENT)).as_bool() {
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_PAINT => {
                #[allow(static_mut_refs)]
                if let Some(s) = SCENE.as_ref() {
                    let r = draw(s);
                    FRAMES += 1;
                    if FRAMES <= 2 || r.is_err() && FRAMES % 60 == 0 {
                        log(&format!("frame {}: {}", FRAMES, r.map(|_| "ok".to_string()).unwrap_or_else(|e| e.message())));
                    }
                }
                let _ = ValidateRect(Some(hwnd), None);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    unsafe fn stops(rt: &ID2D1RenderTarget, s: &[(f32, D2D1_COLOR_F)]) -> Result<ID2D1GradientStopCollection> {
        let v: Vec<D2D1_GRADIENT_STOP> = s.iter().map(|(p, c)| D2D1_GRADIENT_STOP { position: *p, color: *c }).collect();
        rt.CreateGradientStopCollection(&v, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP)
    }

    unsafe fn linear(rt: &ID2D1RenderTarget, a: Vector2, b: Vector2, s: &[(f32, D2D1_COLOR_F)]) -> Result<ID2D1LinearGradientBrush> {
        let st = stops(rt, s)?;
        rt.CreateLinearGradientBrush(&D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: a, endPoint: b }, None, &st)
    }

    unsafe fn radial(rt: &ID2D1RenderTarget, c: Vector2, rx: f32, ry: f32, s: &[(f32, D2D1_COLOR_F)]) -> Result<ID2D1RadialGradientBrush> {
        let st = stops(rt, s)?;
        rt.CreateRadialGradientBrush(
            &D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES { center: c, gradientOriginOffset: pt(0.0, 0.0), radiusX: rx, radiusY: ry },
            None,
            &st,
        )
    }

    fn surface(x: f32, t: f32, w: f32, base: f32, amp: f32) -> f32 {
        // two waves whose periods divide the loop: seamless
        base + amp * (TAU * (x / (w * 0.55) - t / 3.0)).sin() + amp * 0.45 * (TAU * (x / (w * 0.31) + t / 2.0)).sin()
    }

    unsafe fn draw(s: &Scene) -> Result<()> {
        draw_at(s, s.start.elapsed().as_secs_f32() % LOOP)
    }

    unsafe fn draw_at(s: &Scene, t: f32) -> Result<()> {
        let rt = &s.rt;
        let (w, h) = (s.w, s.h);
        rt.BeginDraw();
        rt.SetTransform(&Matrix3x2::identity());
        rt.Clear(Some(&s.bg));

        // the dark glass card, like the installer's side panel
        let m = 2.0;
        let card = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: m, top: m, right: w - m, bottom: h - m }, radiusX: 14.0, radiusY: 14.0 };
        let cardb = linear(rt, pt(0.0, 0.0), pt(0.0, h), &[(0.0, col(26, 35, 54, 1.0)), (1.0, col(11, 15, 23, 1.0))])?;
        rt.FillRoundedRectangle(&card, &cardb);
        let glow = radial(rt, pt(w * 0.5, h * 0.62), w * 0.42, h * 0.8, &[(0.0, col(246, 120, 180, 0.30)), (1.0, col(246, 120, 180, 0.0))])?;
        rt.FillRoundedRectangle(&card, &glow);

        // the liquid's surface
        let base = h * 0.70;
        let amp = h * 0.035;
        let cx = w * 0.5;
        let sy = surface(cx, t, w, base, amp);
        let slope = (surface(cx + 8.0, t, w, base, amp) - surface(cx - 8.0, t, w, base, amp)) / 16.0;

        // the soap bar floats: rides the surface, tilts with it, breathes a little
        let size = h * 0.78;
        let bob = sy - size * 0.24;   // the bar sits a little way into the liquid
        let ang = slope.atan().to_degrees() * 0.6 + 1.5 * (TAU * t / 3.0).sin();
        let breathe = 1.0 + 0.015 * (TAU * t / 3.0 + 1.0).sin();
        // soft shadow under it, inside the liquid
        let sh = radial(rt, pt(cx, sy + h * 0.06), size * 0.42, h * 0.07, &[(0.0, col(60, 8, 40, 0.45)), (1.0, col(60, 8, 40, 0.0))])?;
        rt.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, sy + h * 0.06), radiusX: size * 0.42, radiusY: h * 0.07 }, &sh);
        let tr = Matrix3x2::scale(breathe, breathe) * Matrix3x2::rotation(ang) * Matrix3x2::translation(cx, bob);
        rt.SetTransform(&tr);
        let half = size / 2.0;
        rt.DrawBitmap(&s.icon, Some(&D2D_RECT_F { left: -half, top: -half, right: half, bottom: half }), 1.0, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
        // light sweeping across the glass, kept to the bar's face (as drawn in the icon)
        let (fw, fh) = (size * 0.80, size * 0.52);
        // the icon draws the bar 3% above centre, then turns it 9 degrees clockwise
        rt.SetTransform(&(Matrix3x2::translation(0.0, -size * 0.03) * Matrix3x2::rotation(9.0) * tr));
        let ph = (t % 3.0) / 3.0;
        let sx = -fw * 0.9 + ph * fw * 2.4;
        let sweep = linear(rt, pt(sx - fw * 0.18, -fh * 0.5), pt(sx + fw * 0.18, fh * 0.5), &[(0.0, col(255, 255, 255, 0.0)), (0.5, col(255, 255, 255, 0.32)), (1.0, col(255, 255, 255, 0.0))])?;
        let face = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: -fw / 2.0, top: -fh / 2.0, right: fw / 2.0, bottom: fh / 2.0 }, radiusX: fh * 0.22, radiusY: fh * 0.22 };
        rt.FillRoundedRectangle(&face, &sweep);
        rt.SetTransform(&Matrix3x2::identity());

        // the liquid in front, see-through, so the bar sits in it
        let geo = s.factory.CreatePathGeometry()?;
        let sink = geo.Open()?;
        sink.BeginFigure(pt(m, h - m), D2D1_FIGURE_BEGIN_FILLED);
        let steps = 48;
        for i in 0..=steps {
            let x = m + (w - 2.0 * m) * i as f32 / steps as f32;
            sink.AddLine(pt(x, surface(x, t, w, base, amp)));
        }
        sink.AddLine(pt(w - m, h - m));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
        let liquid = linear(rt, pt(0.0, base - amp * 2.0), pt(0.0, h), &[(0.0, col(246, 140, 190, 0.42)), (1.0, col(120, 30, 84, 0.78))])?;
        rt.PushAxisAlignedClip(&D2D_RECT_F { left: m, top: m, right: w - m, bottom: h - m }, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        rt.FillGeometry(&geo, &liquid, None);
        // its surface catches the light
        let line = rt.CreateSolidColorBrush(&col(255, 220, 238, 0.55), None)?;
        let edge = s.factory.CreatePathGeometry()?;
        let es = edge.Open()?;
        es.BeginFigure(pt(m, surface(m, t, w, base, amp)), D2D1_FIGURE_BEGIN_FILLED);
        for i in 1..=steps {
            let x = m + (w - 2.0 * m) * i as f32 / steps as f32;
            es.AddLine(pt(x, surface(x, t, w, base, amp)));
        }
        es.EndFigure(windows::Win32::Graphics::Direct2D::Common::D2D1_FIGURE_END_OPEN);
        es.Close()?;
        rt.DrawGeometry(&edge, &line, 1.5, None);

        // glass bubbles rising through the liquid, each looping in step with the rest
        let bubbles = [(0.16, 7.0, 0.00), (0.31, 4.5, 0.35), (0.69, 6.0, 0.62), (0.84, 9.0, 0.18), (0.58, 3.5, 0.80)];
        for (fx, r, phase) in bubbles {
            let k = ((t / LOOP) * 2.0 + phase) % 1.0;
            let x = w * fx + 4.0 * (TAU * (k * 2.0 + phase)).sin();
            let top = surface(x, t, w, base, amp) + r + 2.0;
            let y = (h - m - r) - k * ((h - m - r) - top);
            let a = (k * 6.0).min(1.0) * ((1.0 - k) * 6.0).min(1.0);
            let rr = r * (h / 150.0).max(0.8);
            let b = radial(rt, pt(x, y), rr, rr, &[(0.0, col(255, 255, 255, 0.05 * a)), (0.7, col(255, 220, 238, 0.12 * a)), (1.0, col(255, 255, 255, 0.55 * a))])?;
            rt.FillEllipse(&D2D1_ELLIPSE { point: pt(x, y), radiusX: rr, radiusY: rr }, &b);
            let hl = rt.CreateSolidColorBrush(&col(255, 255, 255, 0.7 * a), None)?;
            rt.FillEllipse(&D2D1_ELLIPSE { point: pt(x - rr * 0.35, y - rr * 0.35), radiusX: rr * 0.22, radiusY: rr * 0.22 }, &hl);
        }
        rt.PopAxisAlignedClip();

        // the card's glass rim
        let rim = linear(rt, pt(0.0, 0.0), pt(0.0, h), &[(0.0, col(255, 255, 255, 0.28)), (0.5, col(255, 255, 255, 0.06)), (1.0, col(255, 255, 255, 0.12))])?;
        rt.DrawRoundedRectangle(&card, &rim, 1.0, None);
        rt.EndDraw(None, None)
    }
    #[allow(dead_code)]
    fn _unused(_: HSTRING) {}
}
