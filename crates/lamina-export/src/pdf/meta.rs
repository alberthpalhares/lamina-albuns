//! O que o PDF/X-4 pede além das páginas (spec PD, regras 4, 5 e 8): catálogo com o perfil de
//! saída (`OutputIntent`), dicionário de informações e metadados XMP com a identificação
//! PDF/X-4. Datas fixas e identificadores tirados do conteúdo: o mesmo projeto dá o mesmo PDF.

use pdf_writer::types::OutputIntentSubtype;
use pdf_writer::writers::{Catalog, DocumentInfo};
use pdf_writer::{Chunk, Date, Name, Ref, Str, TextStr};

use super::profile::OUTPUT_CONDITION;

/// Data gravada no PDF (a mesma do perfil de cor).
const DATE: &str = "2026-01-01T00:00:00Z";

fn date() -> Date {
    Date::new(2026)
        .month(1)
        .day(1)
        .hour(0)
        .minute(0)
        .second(0)
        .utc_offset_hour(0)
}

/// Catálogo: árvore de páginas, metadados e o perfil de saída.
pub fn catalog(id: Ref, pages: Ref, metadata: Ref, output_icc: Ref) -> Chunk {
    let mut chunk = Chunk::new();
    let mut catalog = chunk.indirect(id).start::<Catalog>();
    catalog.pages(pages).metadata(metadata);
    catalog
        .output_intents()
        .push()
        .subtype(OutputIntentSubtype::PDFX)
        .output_condition_identifier(TextStr(OUTPUT_CONDITION))
        .output_condition(TextStr(OUTPUT_CONDITION))
        .registry_name(TextStr("http://www.color.org"))
        .info(TextStr(OUTPUT_CONDITION))
        .dest_output_profile(output_icc);
    drop(catalog);
    chunk
}

/// Dicionário de informações, com a versão PDF/X.
pub fn info(id: Ref, title: &str) -> Chunk {
    let mut chunk = Chunk::new();
    chunk
        .indirect(id)
        .start::<DocumentInfo>()
        .title(TextStr(title))
        .creator(TextStr("Lâmina"))
        .producer(TextStr("Lâmina"))
        .creation_date(date())
        .modified_date(date())
        .trapped(pdf_writer::types::TrappingStatus::NotTrapped)
        .pair(Name(b"GTS_PDFXVersion"), Str(b"PDF/X-4"));
    chunk
}

/// Metadados XMP (sem compressão, como o padrão pede), com o identificador do documento tirado
/// do conteúdo.
pub fn metadata(id: Ref, title: &str, digest: &[u8; 16]) -> Chunk {
    let xml = xmp(title, &uuid(digest));
    let mut chunk = Chunk::new();
    chunk.metadata(id, xml.as_bytes());
    chunk
}

/// "uuid:6c616d69-…" a partir dos 16 bytes do identificador.
fn uuid(d: &[u8; 16]) -> String {
    let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "uuid:{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Texto seguro dentro do XML.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xmp(title: &str, id: &str) -> String {
    let title = escape(title);
    format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
    xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
    xmlns:pdfxid="http://www.npes.org/pdfx/ns/id/">
   <dc:format>application/pdf</dc:format>
   <dc:title><rdf:Alt><rdf:li xml:lang="x-default">{title}</rdf:li></rdf:Alt></dc:title>
   <xmp:CreateDate>{DATE}</xmp:CreateDate>
   <xmp:ModifyDate>{DATE}</xmp:ModifyDate>
   <xmp:MetadataDate>{DATE}</xmp:MetadataDate>
   <xmp:CreatorTool>Lâmina</xmp:CreatorTool>
   <pdf:Producer>Lâmina</pdf:Producer>
   <pdf:Trapped>False</pdf:Trapped>
   <xmpMM:DocumentID>{id}</xmpMM:DocumentID>
   <xmpMM:InstanceID>{id}</xmpMM:InstanceID>
   <xmpMM:VersionID>1</xmpMM:VersionID>
   <xmpMM:RenditionClass>default</xmpMM:RenditionClass>
   <pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xmp_com_titulo_escapado_e_identificador() {
        let text = xmp("Ana & Beto <3", &uuid(&[0xab; 16]));
        assert!(text.contains("Ana &amp; Beto &lt;3"));
        assert!(text.contains("uuid:abababab-abab-abab-abab-abababababab"));
        assert!(text.contains("<pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>"));
    }
}
