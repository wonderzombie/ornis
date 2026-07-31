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

        let ty = match token.chars().next() {
            Some('+') => Self::Filter(With),
            Some('-') => Self::Filter(Without),
            Some('@') => Self::Query(Optional),
            Some('#') => Self::Query(Has),
            Some(c) if c.is_alphabetic() || c == '_' => {
                return Ok((QParam::Query(Required), token));
            }
            Some(c) => bail!("unrecognized sigil {c:?} in {token:?}"),
            None => bail!("empty query token"),
        };

        match &token[1..] {
            "" => bail!("bare sigil with no component: {token:?}"),
            rest => Ok((ty, rest)),
        }
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
