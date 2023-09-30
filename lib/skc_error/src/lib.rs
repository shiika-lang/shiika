mod report_builder;
pub use ariadne::Label;
use ariadne::{Report, ReportBuilder, ReportKind, Source};
use report_builder::Builder;
use shiika_ast::LocationSpan;
use std::fs;
use std::ops::Range;

type AriadneSpan<'a> = (&'a String, Range<usize>);

/// Helper for building report with ariadne crate.
/// (TODO: migrate to `report_builder`)
pub fn build_report<F>(main_msg: String, locs: &LocationSpan, f: F) -> String
where
    F: for<'b> FnOnce(
        ReportBuilder<AriadneSpan<'b>>,
        AriadneSpan<'b>,
    ) -> ReportBuilder<AriadneSpan<'b>>,
{
    if let LocationSpan::Just {
        filepath,
        begin,
        end,
    } = locs
    {
        // ariadne::Id for the file `locs.filepath`
        // ariadne 0.1.5 needs Id: Display (zesterer/ariadne#12)
        let id = format!("{}", filepath.display());
        // ariadne::Span equivalent to `locs`
        let locs_span = (&id, begin.pos..end.pos);

        let src = Source::from(fs::read_to_string(&**filepath).unwrap_or_default());
        let report = f(Report::build(ReportKind::Error, &id, begin.pos), locs_span)
            .with_message(main_msg.clone())
            .finish();

        match std::panic::catch_unwind(|| {
            let mut rendered = vec![];
            report.write((&id, src), &mut rendered).unwrap();
            String::from_utf8_lossy(&rendered).to_string()
        }) {
            Ok(u8str) => u8str,
            Err(e) => {
                println!("[BUG] ariadne crate crashed!");
                dbg!(&e);
                main_msg
            }
        }
    } else {
        // No location information available
        main_msg
    }
}

/// Helper for building report with ariadne crate.
pub fn report_builder() -> crate::report_builder::Builder {
    Builder::new()
}
