//! Implementation of IRIs as per [\[RFC 3987\]](https://tools.ietf.org/html/rfc3987).

use pest::{Parser, error::Error, iterators::Pair};
use regex::Regex;

#[cfg(debug_assertions)]
const _GRAMMAR: &'static str = include_str!("iri_rfc3987.pest");


#[inline]
/// Check whether txt is a valid (absolute or relative) IRI.
pub fn is_valid_iri(txt: &str) -> bool {
    IRI_REGEX.is_match(txt) || IRELATIVE_REF_REGEX.is_match(txt)
}

/// Check whether txt is an absolute IRI.
#[inline]
pub fn is_absolute_iri(txt: &str) -> bool {
    IRI_REGEX.is_match(txt)
}

/// Check whether txt is a relative IRI.
#[inline]
pub fn is_relative_iri(txt: &str) -> bool {
    IRELATIVE_REF_REGEX.is_match(txt)
}


// TODO replace Pest by a pure Regex parsing?
// NB: once the IRI has been validated with
// IRI_REGEX or IRELATIVE_REF_REGEX,
// spliting it into its different part is relatively trivial
// (rsplit by #, then rsplit by ?, then split by /)

#[derive(Parser)]
#[grammar = "term/iri_rfc3987.pest"]
pub struct IriParser;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParsedIri<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: Vec<&'a str>,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

// NB: path complies with the following rules:
// - does not contain the seperators ('/')
// - its first element is '' if the path starts with '/'
// - its last element is "" if the path ends with a '/'

impl<'a> ParsedIri<'a> {
    pub fn new(txt: &'a str) -> Result<ParsedIri<'a>, Error<Rule>> {
        let mut pi = ParsedIri::default();
        pi.fill_with(IriParser::parse(Rule::main, txt)?.next().unwrap());
        Ok(pi)
    }

    fn fill_with(&mut self, pair: Pair<'a, Rule>) {
        for subpair in pair.into_inner() {
            match subpair.as_rule() {
                Rule::iri => {
                    self.fill_with(subpair);
                }
                Rule::irelative_ref => {
                    self.fill_with(subpair);
                }
                Rule::scheme => {
                    debug_assert!(self.scheme.is_none());
                    self.scheme = Some(subpair.as_str());
                }
                Rule::ihier_part |
                Rule::irelative_part => {
                    self.fill_with(subpair);
                }
                Rule::iquery => {
                    debug_assert!(self.query.is_none());
                    self.query = Some(subpair.as_str());
                }
                Rule::ifragment => {
                    debug_assert!(self.fragment.is_none());
                    self.fragment = Some(subpair.as_str());
                }
                Rule::iauthority => {
                    debug_assert!(self.authority.is_none());
                    self.authority = Some(subpair.as_str());
                }
                Rule::ipath_abempty => {
                    if subpair.as_str().len() > 0 {
                        self.path.push("");
                        self.fill_with(subpair);
                    }
                }
                Rule::ipath_absolute => {
                    self.path.push("");
                    self.fill_with(subpair);
                }
                Rule::ipath_noscheme |
                Rule::ipath_rootless => {
                    self.fill_with(subpair);
                }
                Rule::ipath_empty => {
                }
                Rule::isegment |
                Rule::isegment_nz |
                Rule::isegment_nz_nc => {
                    self.path.push(subpair.as_str());
                }
                Rule::EOI => {}
                _ => panic!(format!("Can't handle rule {:?}", subpair.as_rule()))
            }
        }
    }

    pub fn is_absolute(&self) -> bool {
        self.scheme.is_some()
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        if let Some(scheme) = self.scheme {
            ret.push_str(scheme);
            ret.push_str(":");
        }
        if let Some(authority) = self.authority {
            ret.push_str("//");
            ret.push_str(authority);
        }
        ret.push_str(&self.path.join("/"));
        if let Some(query) = self.query {
            ret.push_str("?");
            ret.push_str(query)
        }
        if let Some(fragment) = self.fragment {
            ret.push_str("#");
            ret.push_str(fragment)
        }
        ret
    }

    pub fn join(&self, iri_ref: &ParsedIri<'a>) -> ParsedIri<'a> {
        let (scheme, authority, query, fragment);
        let mut path;
        if iri_ref.scheme.is_some() {
            scheme = iri_ref.scheme;
            authority = iri_ref.authority;
            path = iri_ref.path.clone();
            remove_dot_segments(&mut path);
            query = iri_ref.query;
        } else {
            scheme = self.scheme;
            if iri_ref.authority.is_some() {
                authority = iri_ref.authority;
                path = iri_ref.path.clone();
                remove_dot_segments(&mut path);
                query = iri_ref.query;
            } else {
                authority = self.authority;
                if iri_ref.path.len() == 0 {
                    path = self.path.clone();
                    query = iri_ref.query.or(self.query);
                } else {
                    if iri_ref.path[0] == "" {
                        path = iri_ref.path.clone();
                        remove_dot_segments(&mut path);
                    } else {
                        path = merge(&self, &iri_ref.path);
                        remove_dot_segments(&mut path);
                    }
                    query = iri_ref.query;
                }
            }
        }
        fragment = iri_ref.fragment;
        ParsedIri{scheme, authority, path, query, fragment}
    }
}

fn merge<'a> (base: &ParsedIri<'a>, path: &Vec<&'a str>) -> Vec<&'a str> {
    let mut v = Vec::new();
    if base.authority.is_some() && base.path.len() == 0 {
        v.push("");  // resulting path must have a leading '/'
    }
    v.extend(base.path.iter().take(base.path.len()-1).map(|txt| *txt));
    v.extend(path.iter().map(|txt| *txt));
    v
}

fn remove_dot_segments(path: &mut Vec<&str>) {
    if path.len() == 0 {
        return;
    }
    let mut i = 0;
    let last = path[path.len()-1];
    if last == "." || last == ".." {
        path.push("");
    }
    while i < path.len() {
        if path[i] == "." {
            path.remove(i);
        } else if path[i] == ".." {
            if i != 0 && (i != 1 || path[0] != "") {
                path.remove(i-1);
                i -= 1;
            }
            path.remove(i);
        } else {
            i += 1;
        }
    }
}

lazy_static! {
    static ref IRI_REGEX: Regex = Regex::new(r"(?x)^
        #scheme
        [A-Za-z] [-A-Za-z0-9+.]*
        :
        #ihier_part
        ( #iauthority + ipath_abempty
          //
          ( # iuserinfo
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:]
          |
            %[0-9a-fA-F]{2}
          )*
          @
          )?
          # ihost
          ( # ip_literal
             \[
            ( # ipv6address
              (
                ([0-9a-fA-F]{1,4}:){6}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                ::
                ([0-9a-fA-F]{1,4}:){5}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                ([0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:){4}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,1}:[0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:){3}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,2}:[0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:){2}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,3}:[0-9a-fA-F]{1,4})?
                ::
                [0-9a-fA-F]{1,4}:
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,4}:[0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,5}:[0-9a-fA-F]{1,4})?
                ::
                [0-9a-fA-F]{1,4}
              |
                (([0-9a-fA-F]{1,4}:){0,6}:[0-9a-fA-F]{1,4})?
                ::
              )
            | # ipvfuture
              v[0-9a-fA-F]+ \. [-A-Za-z0-9._~!$&'()*+,;=:]+
            )
             \]
          | # ipv4address
            ([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5])) (\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3}
          | # ireg_name
              ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=]
              | %[0-9a-fA-F]{2}
              )*
          )
          (
            :
            [0-9]* # port
          )?
          #ipath_abempty
          (
            /
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
            | %[0-9a-fA-F]{2}
            )*
          )*
        | #ipath_absolute
          /
          (
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
            | %[0-9a-fA-F]{2}
            )*
            (
              /
              ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
              | %[0-9a-fA-F]{2}
              )*
            )*
          )?
        | #ipath_rootless
          ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
          | %[0-9a-fA-F]{2}
          )+
          (
            /
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
            | %[0-9a-fA-F]{2}
            )*
          )*
        )? # optional because of ipath_empty
        ( # ?iquery
          \?
          ([-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@'\u{E000}-\u{F8FF}\u{F0000}-\u{FFFFD}\u{100000}-\u{10FFFD}/?])*
        )?
        ( # #ifragment
          \#
          ([-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@/?])*
        )?
    $").unwrap();

    static ref IRELATIVE_REF_REGEX: Regex = Regex::new(r"(?x)^
        #irelative_part
        ( #iauthority + ipath_abempty
          //
          ( # iuserinfo
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:]
          |
            %[0-9a-fA-F]{2}
          )*
          @
          )?
          # ihost
          ( # ip_literal
             \[
            ( # ipv6address
              (
                ([0-9a-fA-F]{1,4}:){6}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                ::
                ([0-9a-fA-F]{1,4}:){5}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                ([0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:){4}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,1}:[0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:){3}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,2}:[0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:){2}
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,3}:[0-9a-fA-F]{1,4})?
                ::
                [0-9a-fA-F]{1,4}:
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,4}:[0-9a-fA-F]{1,4})?
                ::
                ([0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}|([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))(\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3})
              |
                (([0-9a-fA-F]{1,4}:){0,5}:[0-9a-fA-F]{1,4})?
                ::
                [0-9a-fA-F]{1,4}
              |
                (([0-9a-fA-F]{1,4}:){0,6}:[0-9a-fA-F]{1,4})?
                ::
              )
            | # ipvfuture
              v[0-9a-fA-F]+ \. [-A-Za-z0-9._~!$&'()*+,;=:]+
            )
             \]
          | # ipv4address
            ([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5])) (\.([0-9]|([1-9][0-9])|(1[0-9]{2})|(2[0-4][0-9])|(25[0-5]))){3}
          | # ireg_name
              ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=]
              | %[0-9a-fA-F]{2}
              )*
          )
          (
            :
            [0-9]* # port
          )?
          #ipath_abempty
          (
            /
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
            | %[0-9a-fA-F]{2}
            )*
          )*
        | #ipath_absolute
          /
          (
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
            | %[0-9a-fA-F]{2}
            )*
            (
              /
              ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
              | %[0-9a-fA-F]{2}
              )*
            )*
          )?
        | #ipath_noscheme
          ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=@]
          | %[0-9a-fA-F]{2}
          )+
          (
            /
            ( [-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@]
            | %[0-9a-fA-F]{2}
            )*
          )*
        )? # optional because of ipath_empty
        ( # ?iquery
          \?
          ([-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@'\u{E000}-\u{F8FF}\u{F0000}-\u{FFFFD}\u{100000}-\u{10FFFD}/?])*
        )?
        ( # #ifragment
          \#
          ([-A-Za-z0-9._~\u{A0}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFEF}\u{10000}-\u{1FFFD}\u{20000}-\u{2FFFD}\u{30000}-\u{3FFFD}\u{40000}-\u{4FFFD}\u{50000}-\u{5FFFD}\u{60000}-\u{6FFFD}\u{70000}-\u{7FFFD}\u{80000}-\u{8FFFD}\u{90000}-\u{9FFFD}\u{A0000}-\u{AFFFD}\u{B0000}-\u{BFFFD}\u{C0000}-\u{CFFFD}\u{D0000}-\u{DFFFD}\u{E1000}-\u{EFFFD}!$&'()*+,;=:@/?])*
        )?
    $").unwrap();
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn positive() {
        for (txt, parsed) in POSITIVE_IRIS {
            let rpi = ParsedIri::new(txt);
            assert!(rpi.is_ok(), format!("<{}> → {:?}", txt, rpi));
            let pi = rpi.unwrap();
            assert_eq!(pi.is_absolute(), parsed.0);
            assert_eq!(pi.scheme, parsed.1);
            assert_eq!(pi.authority, parsed.2);
            assert_eq!(&pi.path[..], parsed.3);
            assert_eq!(pi.query, parsed.4);
            assert_eq!(pi.fragment, parsed.5);
            assert_eq!(&pi.to_string(), txt);
        }
    }

    #[test]
    fn negative() {
        for txt in NEGATIVE_IRIS {
            let rpi = ParsedIri::new(txt);
            assert!(rpi.is_err(), format!("<{}> → {:?}", txt, rpi));
        }
    }

    #[test]
    fn relative() {
        let base = ParsedIri::new("http://a/b/c/d;p?q").unwrap();
        for (rel, abs) in RELATIVE_IRIS {
            let rel = ParsedIri::new(rel).unwrap();
            let gpt = base.join(&   rel);
            assert_eq!(&gpt.to_string(), abs);
        }
    }

    #[test]
    fn regex_abs() {
        for (txt, parsed) in POSITIVE_IRIS {
            assert_eq!(IRI_REGEX.is_match(txt), parsed.0);
        }
        for txt in NEGATIVE_IRIS {
            assert!(!IRI_REGEX.is_match(txt));
        }
    }

    #[test]
    fn regex_rel() {
        for (txt, parsed) in POSITIVE_IRIS {
            assert_eq!(IRELATIVE_REF_REGEX.is_match(txt), !parsed.0);
        }
        for txt in NEGATIVE_IRIS {
            assert!(!IRELATIVE_REF_REGEX.is_match(txt));
        }
    }

    const POSITIVE_IRIS: &[(&str, (bool, Option<&str>, Option<&str>, &[&str], Option<&str>, Option<&str>))] = &[
        ("http:",
            (true, Some("http"), None, &[], None, None)),
        ("http://example.org",
            (true, Some("http"), Some("example.org"), &[], None, None)),
        ("http://127.0.0.1",
            (true, Some("http"), Some("127.0.0.1"), &[], None, None)),
        ("http://[::]",
            (true, Some("http"), Some("[::]"), &[], None, None)),
        ("http://%0D",
            (true, Some("http"), Some("%0D"), &[], None, None)),
        ("http://example.org/",
            (true, Some("http"), Some("example.org"), &["", ""], None, None)),
        ("http://éxample.org/",
            (true, Some("http"), Some("éxample.org"), &["", ""], None, None)),
        ("http://user:pw@example.org:1234/",
            (true, Some("http"), Some("user:pw@example.org:1234"), &["", ""], None, None)),
        ("http://example.org/foo/bar/baz",
            (true, Some("http"), Some("example.org"), &["", "foo", "bar", "baz"], None, None)),
        ("http://example.org/foo/bar/",
            (true, Some("http"), Some("example.org"), &["", "foo", "bar", ""], None, None)),
        ("http://example.org/foo/bar/bàz",
            (true, Some("http"), Some("example.org"), &["", "foo", "bar", "bàz"], None, None)),
        ("http://example.org/foo/.././/bar",
            (true, Some("http"), Some("example.org"), &["", "foo", "..", ".", "", "bar"], None, None)),
        ("http://example.org/!$&'()*+,=:@/foo%0D",
            (true, Some("http"), Some("example.org"), &["", "!$&'()*+,=:@", "foo%0D"], None, None)),
        ("http://example.org/?abc",
            (true, Some("http"), Some("example.org"), &["", ""], Some("abc"), None)),
        ("http://example.org/?!$&'()*+,=:@/?\u{E000}",
            (true, Some("http"), Some("example.org"), &["", ""], Some("!$&'()*+,=:@/?\u{E000}"), None)),
        ("http://example.org/#def",
            (true, Some("http"), Some("example.org"), &["", ""], None, Some("def"))),
        ("http://example.org/?abc#def",
            (true, Some("http"), Some("example.org"), &["", ""], Some("abc"), Some("def"))),
        ("tag:abc/def",
            (true, Some("tag"), None, &["abc", "def"], None, None)),
        ("tag:",
            (true, Some("tag"), None, &[], None, None)),

        ("foo",
            (false, None, None, &["foo"], None, None)),
        ("..",
            (false, None, None, &[".."], None, None)),
        ("//example.org",
            (false, None, Some("example.org"), &[], None, None)),
        ("?",
            (false, None, None, &[], Some(""), None)),
        ("#",
            (false, None, None, &[], None, Some(""))),
        ("?#",
            (false, None, None, &[], Some(""), Some(""))),
    ];

    const NEGATIVE_IRIS: &[&str] = &[
        "http://[/",
        "http://a/[",
        "http://a/]",
        "http://a/|",
        "http://a/ ",
        "http://a/\u{E000}",
        "[",
        "]",
        "|",
        " ",
        "\u{E000}",
    ];

    const RELATIVE_IRIS: &[(&str, &str)] = &[
        // all relative iris are resolved agains http://a/b/c/d;p?q
        // normal examples from https://tools.ietf.org/html/rfc3986#section-5.4.1
        ("g:h"           , "g:h"),
        ("g"             , "http://a/b/c/g"),
        ("./g"           , "http://a/b/c/g"),
        ("g/"            , "http://a/b/c/g/"),
        ("/g"            , "http://a/g"),
        ("//g"           , "http://g"),
        ("?y"            , "http://a/b/c/d;p?y"),
        ("g?y"           , "http://a/b/c/g?y"),
        ("#s"            , "http://a/b/c/d;p?q#s"),
        ("g#s"           , "http://a/b/c/g#s"),
        ("g?y#s"         , "http://a/b/c/g?y#s"),
        (";x"            , "http://a/b/c/;x"),
        ("g;x"           , "http://a/b/c/g;x"),
        ("g;x?y#s"       , "http://a/b/c/g;x?y#s"),
        (""              , "http://a/b/c/d;p?q"),
        ("."             , "http://a/b/c/"),
        ("./"            , "http://a/b/c/"),
        (".."            , "http://a/b/"),
        ("../"           , "http://a/b/"),
        ("../g"          , "http://a/b/g"),
        ("../.."         , "http://a/"),
        ("../../"        , "http://a/"),
        ("../../g"       , "http://a/g"),
        // abnormal example from https://tools.ietf.org/html/rfc3986#section-5.4.2
        ("../../../g"    , "http://a/g"),
        ("../../../../g" , "http://a/g"),
        ("/./g"          , "http://a/g"),
        ("/../g"         , "http://a/g"),
        ("g."            , "http://a/b/c/g."),
        (".g"            , "http://a/b/c/.g"),
        ("g.."           , "http://a/b/c/g.."),
        ("..g"           , "http://a/b/c/..g"),
        ("./../g"        , "http://a/b/g"),
        ("./g/."         , "http://a/b/c/g/"),
        ("g/./h"         , "http://a/b/c/g/h"),
        ("g/../h"        , "http://a/b/c/h"),
        ("g;x=1/./y"     , "http://a/b/c/g;x=1/y"),
        ("g;x=1/../y"    , "http://a/b/c/y"),
        ("g?y/./x"       , "http://a/b/c/g?y/./x"),
        ("g?y/../x"      , "http://a/b/c/g?y/../x"),
        ("g#s/./x"       , "http://a/b/c/g#s/./x"),
        ("g#s/../x"      , "http://a/b/c/g#s/../x"),
    ];

}