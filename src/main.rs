use futures::stream::{self, StreamExt};
use lopdf::{dictionary, Document, Object, Stream};
use reqwest::Client;
use std::fs::File;

const BASE_URL: &str = "https://orangewebsupport.co.in/assets/files/ebook/Touchpad_Aiv3.0_417/Book10/Touchpad_AI_Ebook-10_V3.0/resources/book/file-page{}.jpg";
const TOTAL_PAGES: u32 = 518;
const CONCURRENCY: usize = 20; // High speed parallel download

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    println!("🚀 Starting download of {} pages...", TOTAL_PAGES);

    // 1. Download pages in parallel
    let mut pages: Vec<(u32, Vec<u8>)> = stream::iter(1..=TOTAL_PAGES)
        .map(|page| {
            let client = client.clone();
            async move {
                let url = BASE_URL.replace("{}", &page.to_string());
                let resp = client.get(url).send().await.ok()?;
                let bytes = resp.bytes().await.ok()?;

                if bytes.len() < 1000 {
                    return None;
                } // Skip broken/empty images

                println!("✅ Downloaded page {}", page);
                Some((page, bytes.to_vec()))
            }
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|x| async { x })
        .collect()
        .await;

    // Ensure they are in the correct order for the PDF
    pages.sort_by_key(|(p, _)| *p);

    if pages.len() == 0 {
        panic!("Failed to download any pages.");
    }

    // 2. Build PDF structure
    println!("📄 Generating PDF for {} pages...", pages.len());
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let catalog_id = doc.new_object_id();
    let mut page_refs = Vec::new();

    for (page_num, jpg) in pages {
        // Fast header-only dimension check
        let (width, height) = match imagesize::blob_size(&jpg) {
            Ok(dim) => (dim.width, dim.height),
            Err(_) => (2480, 3508), // Default A4 fallback
        };

        let img_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        // Embed original JPEG bytes (no re-encoding)
        doc.objects.insert(
            img_id,
            Object::Stream(Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => width as i64,
                    "Height" => height as i64,
                    "ColorSpace" => "DeviceRGB",
                    "BitsPerComponent" => 8,
                    "Filter" => "DCTDecode"
                },
                jpg,
            )),
        );

        // PDF operators: Scale image to fill the default page area
        let content_data = b"q 595 0 0 842 0 0 cm /Im0 Do Q".to_vec();
        doc.objects.insert(
            content_id,
            Object::Stream(Stream::new(dictionary! {}, content_data)),
        );

        // Assemble Page
        doc.objects.insert(
            page_id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => img_id } },
                "Contents" => content_id
            }),
        );

        page_refs.push(Object::Reference(page_id));
    }

    // Catalog & Root Setup
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_refs.clone(),
            "Count" => page_refs.len() as i64
        }),
    );

    doc.objects.insert(
        catalog_id,
        Object::Dictionary(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id
        }),
    );

    doc.trailer.set("Root", catalog_id);

    let output_name = "Touchpad_AI_Class10_Complete.pdf";
    doc.save_to(&mut File::create(output_name)?)?;

    println!("🎉 All {} pages saved to {}", TOTAL_PAGES, output_name);
    Ok(())
}
