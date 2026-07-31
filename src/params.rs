use anyhow::bail;
use iced::overlay::menu::default;

use crate::rpc::QueryParams;

pub enum QData {
    Required,
    Optional,
    Has,
}

pub enum QFilter {
    With,
    Without,
}

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

pub(crate) fn parse(words: &[&str]) -> Result<QueryParams, anyhow::Error> {
    let mut qp = QueryParams::default();

    for w in words {
        let (ty, name) = QParam::parse(w)?;
        let bucket = match ty {
            QParam::Filter(QFilter::With) => &mut qp.filter.with,
            QParam::Filter(QFilter::Without) => &mut qp.filter.without,
            QParam::Query(QData::Required) => &mut qp.data.components,
            QParam::Query(QData::Optional) => &mut qp.data.option,
            QParam::Query(QData::Has) => &mut qp.data.has,
        };
        bucket.push(name.to_owned());
    }

    Ok(qp)
}
