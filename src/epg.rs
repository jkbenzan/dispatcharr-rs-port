use quick_xml::events::Event;
use quick_xml::reader::Reader;
use sea_orm::DatabaseConnection;
use std::error::Error;

pub async fn refresh_all_guides(_db: &DatabaseConnection) -> Result<(), Box<dyn Error>> {
    println!("Starting XMLTV EPG Parsing...");

    // Simulated XML fetching
    let xml_data = r#"
        <tv generator-info-name="Dispatcharr">
            <channel id="CNN">
                <display-name>CNN HD</display-name>
            </channel>
            <programme start="20260416000000 +0000" stop="20260416010000 +0000" channel="CNN">
                <title>News Hour</title>
                <desc>Daily News</desc>
            </programme>
        </tv>
    "#;

    let mut reader = Reader::from_str(xml_data);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_channel = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"channel" => in_channel = true,
                    b"programme" => {
                        // Extract programme details
                    }
                    _ => (),
                }
            }
            Ok(Event::Text(e)) => {
                if in_channel {
                    let _txt = e.unescape().unwrap().into_owned();
                    // Process display-name or other tags
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"channel" => in_channel = false,
                    _ => (),
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
        buf.clear();
    }

    println!("EPG Parsing Complete");
    Ok(())
}