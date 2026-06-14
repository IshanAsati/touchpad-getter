import asyncio
import aiohttp
import img2pdf
import sys

BASE_URL = "https://orangewebsupport.co.in/assets/files/ebook/Touchpad_Aiv3.0_417/Book10/Touchpad_AI_Ebook-10_V3.0/resources/book/file-page{}.jpg"
TOTAL_PAGES = 518
CONCURRENCY = 20

async def download_page(session, semaphore, page):
    async with semaphore:
        url = BASE_URL.format(page)
        try:
            async with session.get(url) as response:
                if response.status == 200:
                    data = await response.read()
                    if len(data) < 1000:
                        return None
                    print(f"✅ Downloaded page {page}")
                    return (page, data)
                else:
                    return None
        except Exception as e:
            print(f"❌ Error downloading page {page}: {e}")
            return None

async def main():
    print(f"🚀 Starting download of {TOTAL_PAGES} pages...")
    
    semaphore = asyncio.Semaphore(CONCURRENCY)
    
    async with aiohttp.ClientSession() as session:
        tasks = [download_page(session, semaphore, page) for page in range(1, TOTAL_PAGES + 1)]
        results = await asyncio.gather(*tasks)
        
    pages = [res for res in results if res is not None]
    pages.sort(key=lambda x: x[0])
    
    if not pages:
        print("Failed to download any pages.")
        sys.exit(1)
        
    print(f"📄 Generating PDF for {len(pages)} pages...")
    
    image_bytes_list = [img_data for _, img_data in pages]
    
    output_name = "Touchpad_AI_Class10_Complete.pdf"
    
    # Scale to A4
    layout_fun = img2pdf.get_layout_fun(
        (img2pdf.mm_to_pt(210), img2pdf.mm_to_pt(297))
    )
    
    with open(output_name, "wb") as f:
        f.write(img2pdf.convert(image_bytes_list, layout_fun=layout_fun))
        
    print(f"🎉 All {len(pages)} pages saved to {output_name}")

if __name__ == "__main__":
    asyncio.run(main())
