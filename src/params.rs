use anyhow::bail;
use log::trace;

use crate::rpc::QueryParams;

#[derive(Debug)]
pub enum QData {
    Required,
    Optional,
    Has,
}

#[derive(Debug)]
pub enum QFilter {
    With,
    Without,
}

#[derive(Debug)]
pub enum QParam {
    Query(QData),
    Filter(QFilter),
}

impl QParam {
    pub(crate) fn parse(token: &str) -> Result<(Self, &str), anyhow::Error> {
        use QData::*;
        use QFilter::*;

        let (ty, rest) = match token.chars().next() {
            Some('+') => (Self::Filter(With), &token[1..]),
            Some('-') => (Self::Filter(Without), &token[1..]),
            Some('@') => (Self::Query(Optional), &token[1..]),
            Some('#') => (Self::Query(Has), &token[1..]),
            Some(_) => (QParam::Query(Required), token),
            None => bail!("empty query token"),
        };

        if rest.is_empty() {
            bail!("bare sigil with no component: {token:?}");
        }

        Ok((ty, rest))
    }
}

pub(crate) fn collect(items: impl IntoIterator<Item = (QParam, String)>) -> QueryParams {
    let mut qp = QueryParams::default();

    for (ty, name) in items {
        let bucket = match ty {
            QParam::Filter(QFilter::With) => &mut qp.filter.with,
            QParam::Filter(QFilter::Without) => &mut qp.filter.without,
            QParam::Query(QData::Required) => &mut qp.data.components,
            QParam::Query(QData::Optional) => &mut qp.data.option,
            QParam::Query(QData::Has) => &mut qp.data.has,
        };
        trace!("parsed: {:?}", (ty, &name));
        bucket.push(name.to_owned());
    }
    qp
}

#[cfg(test)]
mod tests {
    use crate::rpc::{QueryData, QueryFilter};

    use super::*;

    use std::assert_matches;

    #[test]
    fn test_parse_token_simple() {
        let token = "tiles::TileIdx";
        let (p, s) = QParam::parse(&token).expect("could not parse elementary token {token}");
        assert_matches!(p, QParam::Query(QData::Required));
        assert_eq!(token, s);
    }

    #[test]
    fn test_parse_token_variety() {
        let token = "@tiles::TileIdx";
        let (p, s) = QParam::parse(&token).expect("expected {token} to parse");
        assert_matches!(p, QParam::Query(QData::Optional));
        assert_eq!(&token[1..], s);

        let token = "+tiles::TileIdx";
        let (p, s) = QParam::parse(&token).expect("expected {token} to parse");
        assert_matches!(p, QParam::Filter(QFilter::With));
        assert_eq!(&token[1..], s);

        let token = "-tiles::TileIdx";
        let (p, s) = QParam::parse(&token).expect("expeceted {token} to parse");
        assert_matches!(p, QParam::Filter(QFilter::Without));
        assert_eq!(&token[1..], s);
    }

    #[test]
    fn test_parse_token_passthrough() {
        // Expect this to be unchanged.
        let token = "::tiles::TileIdx";
        let (p, s) = QParam::parse(&token).expect("expeceted {token} to parse");
        assert_matches!(
            p,
            QParam::Query(QData::Required),
            "unrecognized tokens should pass through unaltered"
        );
        assert_eq!(token, s);
    }

    #[test]
    fn test_collect_tokens_simple() {
        let tokens = vec![
            (
                QParam::Query(QData::Required),
                "wanderrust::tiles::TileIdx".into(),
            ),
            (
                QParam::Filter(QFilter::With),
                "wanderrust::tiles::Revealed".into(),
            ),
        ];
        let collected = collect(tokens);

        let expected = QueryParams {
            data: QueryData {
                components: vec!["wanderrust::tiles::TileIdx".into()],
                ..Default::default()
            },
            filter: QueryFilter {
                with: vec!["wanderrust::tiles::Revealed".into()],
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(expected, collected);
    }

    #[test]
    fn test_collect_tokens_variety() {
        let tokens = vec![
            (QParam::Query(QData::Required), "Actor".into()),
            (QParam::Query(QData::Required), "Combatant".into()),
            (QParam::Query(QData::Has), "HasEquipped".into()),
            (QParam::Query(QData::Optional), "Alerted".into()),
            ((QParam::Filter(QFilter::With)), "AgentOfGrid".into()),
            ((QParam::Filter(QFilter::Without)), "NextPos".into()),
            ((QParam::Filter(QFilter::Without)), "MapTile".into()),
        ];
        let collected = collect(tokens);

        let expected = QueryParams {
            data: QueryData {
                // Unfortunately ordering here must match the order of the input above.
                components: vec!["Actor".into(), "Combatant".into()],
                has: vec!["HasEquipped".into()],
                option: vec!["Alerted".into()],
            },
            filter: QueryFilter {
                with: vec!["AgentOfGrid".into()],
                // Unfortunately, again, ordering here must match the order of the input above.
                without: vec!["NextPos".into(), "MapTile".into()],
            },
            ..Default::default()
        };

        assert_eq!(expected, collected);
    }
}
