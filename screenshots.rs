use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use image::{ImageBuffer, Rgba};

pub fn capture() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    unsafe {
        // Get desktop window and DC
        let hwnd = GetDesktopWindow();
        let hdc = GetDC(hwnd);
        let mem_dc = CreateCompatibleDC(hdc);
        
        // Get screen dimensions
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        
        // Create bitmap
        let bitmap = CreateCompatibleBitmap(hdc, width, height);
        SelectObject(mem_dc, bitmap);
        
        // Copy screen to bitmap
        BitBlt(mem_dc, 0, 0, width, height, hdc, 0, 0, SRCCOPY);
        
        // Get bitmap data
        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default(); 1],
        };
        
        let mut buffer: Vec<u8> = vec![0; (width * height * 4) as usize];
        GetDIBits(mem_dc, bitmap, 0, height as u32, 
                  buffer.as_mut_ptr() as *mut _, &bitmap_info, DIB_RGB_COLORS);
        
        // Convert to PNG
        let mut img = ImageBuffer::new(width as u32, height as u32);
        for (i, pixel) in buffer.chunks(4).enumerate() {
            let x = (i as i32 % width) as u32;
            let y = (i as i32 / width) as u32;
            img.put_pixel(x, y, Rgba([pixel[2], pixel[1], pixel[0], 255]));
        }
        
        let mut png_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        
        // Cleanup
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(hwnd, hdc);
        
        Ok(png_data)
    }
}