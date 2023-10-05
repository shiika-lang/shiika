use ariadne::{Label, Report, ReportBuilder, ReportKind, Source};
use shiika_ast::LocationSpan;
use std::fs;

type AriadneSpan<'a> = (&'a String, std::ops::Range<usize>);

pub struct Builder {
    annotations: Vec<(LocationSpan, String)>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            annotations: vec![],
        }
    }

    pub fn annotate(mut self, locs: LocationSpan, msg: String) -> Self {
        self.annotations.push((locs, msg));
        self
    }

    pub fn build(self, main_msg: String, main_locs: &LocationSpan) -> String {
        if let LocationSpan::Just {
            filepath, begin, ..
        } = main_locs
        {
            // ariadne::Id for the file `locs.filepath`
            // ariadne 0.1.5 needs Id: Display (zesterer/ariadne#12)
            let id = format!("{}", filepath.display());

            let src = Source::from(fs::read_to_string(&**filepath).unwrap_or_default());
            let mut r: ReportBuilder<AriadneSpan> =
                Report::build(ReportKind::Error, &id, begin.pos);
            for (locs, msg) in self.annotations {
                let LocationSpan::Just { begin, end, .. } = locs else {
                    panic!("got LocationSpan::None");
                };
                let locs_span = (&id, begin.pos..end.pos);
                r.add_label(Label::new(locs_span).with_message(msg));
            }
            let report = r.with_message(main_msg.clone()).finish();

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
}
