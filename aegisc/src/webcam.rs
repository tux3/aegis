use anyhow::{bail, Result};
use image::{ImageBuffer, Rgb};
use rscam::Camera;

fn find_res_or_better(cam: &Camera, res: (u32, u32)) -> Result<(u32, u32)> {
    match cam.resolutions(b"RGB3")? {
        rscam::ResolutionInfo::Discretes(ref v) => {
            for &(w, h) in v {
                if w >= res.0 && h >= res.1 {
                    return Ok((w, h));
                }
            }
            bail!("Couldn't find matching or better discrete resolution")
        }
        rscam::ResolutionInfo::Stepwise { min, max, step } => {
            if ((res.0 - min.0) / step.0) * step.0 + min.0 == res.0
                && ((res.1 - min.1) / step.1) * step.1 + min.1 == res.1
                && max.0 >= res.0
                && max.1 >= res.1
            {
                Ok((res.0, res.1))
            } else {
                bail!("Couldn't find matching or better stepwise resolution")
            }
        }
    }
}

fn find_best_res(cam: &Camera) -> Result<(u32, u32)> {
    for res in [(1280, 1024), (1024, 768), (800, 600), (640, 480)] {
        if let Ok(res) = find_res_or_better(cam, res) {
            return Ok(res);
        }
    }
    bail!("Couldn't find valid reasonable resolution")
}

pub fn capture_webcam_picture() -> Result<ImageBuffer<Rgb<u8>, rscam::Frame>> {
    let mut cam = Camera::new("/dev/video0")?;
    let res = find_best_res(&cam)?;

    cam.start(&rscam::Config {
        interval: (1, 30),
        resolution: res,
        format: b"RGB3",
        ..Default::default()
    })?;

    // Give cheap webcams a few frames to adjust settings :)
    for _ in 0..30 {
        let _ = cam.capture();
    }
    let frame = cam.capture()?;
    let _ = cam.stop();
    let frame = ImageBuffer::from_raw(frame.resolution.0, frame.resolution.1, frame).unwrap();
    Ok(frame)
}
