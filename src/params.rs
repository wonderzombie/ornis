use anyhow::bail;
use log::info;

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

pub(crate) fn parse(words: &[String]) -> Result<QueryParams, anyhow::Error> {
    let mut qp = QueryParams::default();

    info!("words are {words:?}");

    for w in words {
        info!("parsing {w:?}");
        let (ty, name) = QParam::parse(w.as_ref())?;
        let bucket = match ty {
            QParam::Filter(QFilter::With) => &mut qp.filter.with,
            QParam::Filter(QFilter::Without) => &mut qp.filter.without,
            QParam::Query(QData::Required) => &mut qp.data.components,
            QParam::Query(QData::Optional) => &mut qp.data.option,
            QParam::Query(QData::Has) => &mut qp.data.has,
        };
        info!("parsed: {:?}", (ty, name));
        bucket.push(name.to_owned());
    }

    Ok(qp)
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
    fn test_parse_words_simple() {
        let tokens: Vec<String> = vec!["tiles::TileIdx".into(), "+tiles::Revealed".into()];
        let qp = parse(&tokens).expect("expected {tokens:?} to parse");

        let expected = QueryParams {
            data: QueryData {
                components: vec!["tiles::TileIdx".into()],
                ..Default::default()
            },
            filter: QueryFilter {
                with: vec!["tiles::Revealed".into()],
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(expected, qp);
    }

    #[test]
    fn test_parse_words_variety() {
        let tokens: Vec<String> = vec![
            "Actor".into(),
            "#Alerted".into(),
            "-NextPos".into(),
            "+AgentOfGrid".into(),
        ];
        let qp = parse(&tokens).expect("expected a small variety of tokens to parse");

        let expected = QueryParams {
            data: QueryData {
                components: vec!["Actor".into()],
                has: vec!["Alerted".into()],
                option: vec![],
            },
            filter: QueryFilter {
                with: vec!["AgentOfGrid".into()],
                without: vec!["NextPos".into()],
            },
            ..Default::default()
        };

        assert_eq!(expected, qp);
    }

    #[test]
    fn test_parse_words_mixed_order() {
        let tokens: Vec<String> = vec![
            "+AgentOfGrid".into(),
            "Actor".into(),
            "-NextPos".into(),
            "@HasEquipped".into(),
            "+FixedLoot".into(),
            "#Alerted".into(),
            "Combatant".into(),
        ];
        let qp = parse(&tokens).expect("expected a small variety of tokens to parse");

        let expected = QueryParams {
            data: QueryData {
                components: vec!["Actor".into(), "Combatant".into()],
                option: vec!["HasEquipped".into()],
                has: vec!["Alerted".into()],
            },
            filter: QueryFilter {
                with: vec!["AgentOfGrid".into(), "FixedLoot".into()],
                without: vec!["NextPos".into()],
            },
            ..Default::default()
        };

        assert_eq!(expected, qp);
    }
}
