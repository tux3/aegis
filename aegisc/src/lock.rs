use crate::run_as::run_as_root;
use aegislib::command::server::StatusUpdate;
use anyhow::{anyhow, Result};
use framebuffer::{Framebuffer, KdMode};
use image::imageops::FilterType;
use image::{Bgra, ImageBuffer};
use nix::libc::ioctl;
use std::fs::File;
use std::mem::{forget, size_of};
use std::os::unix::io::AsRawFd;
use tracing::{debug, error, info, trace, warn};

fn set_vt_lock_ioctl(lock: bool) -> Result<()> {
    let tty = File::open("/dev/tty0")?; // Requires root (or tty group membership)

    const REGULAR_VT_NUM: u64 = 7; // Cheap assumption... this is fallback best effort code
    const LOCK_TARGET_VT_NUM: u64 = 25; // Probably unused arbitrary VT
    const VT_ACTIVATE: u64 = 0x5606;
    const VT_LOCKSWITCH: u64 = 0x560B;
    const VT_UNLOCKSWITCH: u64 = 0x560C;

    let target = if lock {
        LOCK_TARGET_VT_NUM
    } else {
        REGULAR_VT_NUM
    };
    unsafe {
        if !lock {
            ioctl(tty.as_raw_fd(), VT_UNLOCKSWITCH, 0);
        }
        ioctl(tty.as_raw_fd(), VT_ACTIVATE, target);
        if lock {
            ioctl(tty.as_raw_fd(), VT_LOCKSWITCH, 0);
        }
    }
    Ok(())
}

fn set_vt_lock(lock: bool) -> Result<()> {
    let bool_str = if lock { "1" } else { "0" };
    if let Err(e) = std::fs::write("/sys/aegisk/lock_vt", bool_str) {
        warn!(
            "Failed to write vt_lock file, module may not be running ({})",
            e
        );
        return set_vt_lock_ioctl(lock);
    }
    Ok(())
}

fn get_screenshot() -> Result<ImageBuffer<Bgra<u8>, Vec<u8>>> {
    let mut capturer = captrs::Capturer::new(0).map_err(|s| anyhow!(s))?;
    let mut frame = capturer
        .capture_frame()
        .map_err(|e| anyhow!("Failed to capture frame: {:?}", e))?;
    let geometry = capturer.geometry();
    let frame_data = frame.as_mut_ptr();
    let frame_byte_len = frame.len() * size_of::<captrs::Bgr8>();
    let frame_byte_cap = frame.capacity() * size_of::<captrs::Bgr8>();
    forget(frame);
    let frame =
        unsafe { Vec::from_raw_parts(frame_data as *mut u8, frame_byte_len, frame_byte_cap) };

    Ok(ImageBuffer::<Bgra<u8>, Vec<u8>>::from_raw(geometry.0, geometry.1, frame).unwrap())
}

fn draw_screenshot(mut screen: ImageBuffer<Bgra<u8>, Vec<u8>>) -> Result<()> {
    let mut framebuffer = Framebuffer::new("/dev/fb0")?;
    let mut new_mode = framebuffer.var_screen_info.clone();
    new_mode.xres = screen.width();
    new_mode.xres_virtual = screen.width();
    new_mode.yres = screen.height();
    new_mode.yres_virtual = screen.height();
    trace!(
        "Got screenshot with resolution {}x{}",
        screen.width(),
        screen.height()
    );
    let mode = if Framebuffer::put_var_screeninfo(&framebuffer.device, &new_mode).is_ok() {
        trace!(
            "Requested framebuffer switch to {}x{} resolution",
            new_mode.xres,
            new_mode.yres
        );
        &new_mode
    } else {
        &framebuffer.var_screen_info
    };

    let w = mode.xres;
    let h = mode.yres;
    let line_length = framebuffer.fix_screen_info.line_length;
    let bytespp = mode.bits_per_pixel / 8;
    let mut frame = vec![0u8; (line_length * h) as usize];

    if screen.width() != w || screen.height() != h {
        screen = image::imageops::resize(&screen, w, h, FilterType::Triangle);
    }

    debug!("Drawing screenshot at {}x{} with {} bpp", w, h, bytespp);
    let _ = Framebuffer::set_kd_mode_ex("/dev/tty25", KdMode::Graphics)?;
    for y in 0..h {
        for x in 0..w {
            let idx = (y * line_length + x * bytespp) as usize;
            let p = &screen.get_pixel(x, y).0;
            frame[idx] = p[0];
            frame[idx + 1] = p[1];
            frame[idx + 2] = p[2];
        }
    }
    framebuffer.write_frame(&frame);
    Ok(())
}

pub fn apply_status(status: impl Into<StatusUpdate>) {
    let status = status.into();
    info!("Applying device status: {:?}", status);
    if status.ssh_locked {
        if let Err(e) = run_as_root(vec!["systemctl", "stop", "ssh"]) {
            error!("Failed to lock SSH: {}", e)
        }
    } else if let Err(e) = run_as_root(vec!["systemctl", "start", "ssh"]) {
        error!("Failed to unlock SSH: {}", e)
    }

    let screenshot = if status.vt_locked {
        get_screenshot().ok()
    } else {
        None
    };
    if !status.vt_locked {
        if let Err(e) = Framebuffer::set_kd_mode_ex("/dev/tty25", KdMode::Text) {
            warn!("Failed to switch TTY back to text mode: {}", e)
        }
    }

    if let Err(e) = set_vt_lock(status.vt_locked) {
        error!("Failed to set vt_lock ({})", e);
    }

    if status.vt_locked {
        if let Some(screen) = screenshot {
            if let Err(e) = draw_screenshot(screen) {
                error!("Failed to draw screenshot: {}", e);
            }
        } else {
            warn!("Failed to capture screenshot")
        }
    }
}
