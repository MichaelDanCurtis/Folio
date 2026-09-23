//! Named move containers link source and destination revisions. Revision ids
//! identify records, not pairs; range ids match ends and names match sides.
use roxmltree::Node;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

pub fn is_move(n: Node) -> bool {
    n.has_tag_name((W, "moveFrom")) || n.has_tag_name((W, "moveTo"))
}
pub fn is_boundary(n: Node) -> bool {
    n.tag_name().namespace() == Some(W)
        && matches!(
            n.tag_name().name(),
            "moveFromRangeStart" | "moveFromRangeEnd" | "moveToRangeStart" | "moveToRangeEnd"
        )
}
fn scope(n: Node) -> usize {
    n.ancestors()
        .find(|a| {
            a.tag_name().namespace() == Some(W)
                && matches!(
                    a.tag_name().name(),
                    "body" | "hdr" | "ftr" | "footnote" | "endnote" | "comment" | "txbxContent"
                )
        })
        .map(|a| a.range().start)
        .unwrap_or(0)
}
fn active(n: Node) -> bool {
    is_move(n)
        && !n
            .ancestors()
            .skip(1)
            .any(crate::docx_revisions::is_property)
}
struct Side {
    name: String,
    scope: usize,
    from: bool,
    start: Range<usize>,
    end: Option<Range<usize>>,
    error: Option<String>,
}
#[derive(Clone)]
pub struct Group {
    pub name: String,
    pub story: String,
    pub members: Vec<usize>,
    pub markers: Vec<Range<usize>>,
    error: Option<String>,
}
pub struct Catalog {
    groups: Vec<Group>,
    member_group: BTreeMap<usize, usize>,
    errors: BTreeMap<usize, String>,
}
impl Catalog {
    pub fn read(root: Node) -> Self {
        let mut sides = vec![];
        for start in root.descendants().filter(|n| {
            n.has_tag_name((W, "moveFromRangeStart")) || n.has_tag_name((W, "moveToRangeStart"))
        }) {
            let from = start.has_tag_name((W, "moveFromRangeStart"));
            let id = start.attribute((W, "id"));
            let name = start.attribute((W, "name")).unwrap_or("").to_string();
            let owner = scope(start);
            let ends = root
                .descendants()
                .filter(|n| {
                    n.has_tag_name((
                        W,
                        if from {
                            "moveFromRangeEnd"
                        } else {
                            "moveToRangeEnd"
                        },
                    )) && scope(*n) == owner
                        && id.is_some()
                        && n.attribute((W, "id")) == id
                })
                .collect::<Vec<_>>();
            let starts = root
                .descendants()
                .filter(|n| {
                    n.tag_name() == start.tag_name()
                        && scope(*n) == owner
                        && n.attribute((W, "id")) == id
                })
                .count();
            let mut error = None;
            if name.is_empty()
                || id.is_none()
                || starts != 1
                || ends.len() != 1
                || ends
                    .first()
                    .is_some_and(|n| n.range().start < start.range().end)
            {
                error = Some("The move range has missing, duplicate or reversed endpoints".into());
            }
            if ["colFirst", "colLast", "displacedByCustomXml"]
                .iter()
                .any(|a| start.attribute((W, *a)).is_some())
                || ends
                    .iter()
                    .any(|end| end.attribute((W, "displacedByCustomXml")).is_some())
            {
                error = Some(
                    "Column-scoped or displaced move ranges need additional review support".into(),
                );
            }
            sides.push(Side {
                name,
                scope: owner,
                from,
                start: start.range(),
                end: ends.first().map(|n| n.range()),
                error,
            });
        }
        // ISO disallows overlapping source containers (and destination
        // containers). Do not guess which named move owns ambiguous content.
        for i in 0..sides.len() {
            for j in i + 1..sides.len() {
                if sides[i].scope == sides[j].scope && sides[i].from == sides[j].from {
                    if let (Some(a), Some(b)) = (&sides[i].end, &sides[j].end) {
                        if sides[i].start.end < b.start && sides[j].start.end < a.start {
                            sides[i].error =
                                Some("Overlapping move containers are ambiguous".into());
                            sides[j].error =
                                Some("Overlapping move containers are ambiguous".into());
                        }
                    }
                }
            }
        }
        let mut grouped: HashMap<(usize, String), Vec<usize>> = HashMap::new();
        for (i, s) in sides.iter().enumerate() {
            grouped
                .entry((s.scope, s.name.clone()))
                .or_default()
                .push(i);
        }
        let mut result = Self {
            groups: vec![],
            member_group: BTreeMap::new(),
            errors: BTreeMap::new(),
        };
        let mut side_group = HashMap::new();
        for ((owner, name), indices) in grouped {
            let group = result.groups.len();
            let mut error = indices.iter().find_map(|i| sides[*i].error.clone());
            if indices.len() != 2 || indices.iter().filter(|i| sides[**i].from).count() != 1 {
                error=Some("The named move needs one source range and one linked destination range in this story".into());
            }
            let mut markers = vec![];
            for i in indices {
                side_group.insert(i, group);
                markers.push(sides[i].start.clone());
                if let Some(end) = &sides[i].end {
                    markers.push(end.clone());
                }
            }
            let story = root
                .descendants()
                .find(|n| n.range().start == owner)
                .map(|n| {
                    format!(
                        "{}:{}",
                        n.tag_name().name(),
                        n.attribute((W, "id")).unwrap_or("")
                    )
                })
                .unwrap_or_default();
            if story.starts_with("txbxContent:") {
                error = Some("Moves inside text boxes need additional review support".into());
            }
            result.groups.push(Group {
                name,
                story,
                members: vec![],
                markers,
                error,
            });
        }
        for n in root.descendants().filter(|n| active(*n)) {
            let from = n.has_tag_name((W, "moveFrom"));
            let mark = n.parent().is_some_and(|p| {
                p.has_tag_name((W, "rPr")) && p.parent().is_some_and(|p| p.has_tag_name((W, "pPr")))
            });
            let position = if mark {
                n.ancestors()
                    .find(|p| p.has_tag_name((W, "p")))
                    .map(|p| p.range().end)
            } else {
                None
            };
            let containing = sides
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.scope == scope(n)
                        && s.from == from
                        && s.end.as_ref().is_some_and(|end| {
                            if let Some(pos) = position {
                                s.start.end <= pos && pos <= end.start
                            } else {
                                s.start.end <= n.range().start && n.range().end <= end.start
                            }
                        })
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if containing.len() != 1 {
                result.errors.insert(
                    n.range().start,
                    "Moved content must belong to one uniquely paired named move range".into(),
                );
                for i in containing {
                    result.groups[side_group[&i]].error =
                        Some("A move member belongs to ambiguous containers".into());
                }
                continue;
            }
            let g = side_group[&containing[0]];
            result.groups[g].members.push(n.range().start);
            result.member_group.insert(n.range().start, g);
        }
        for g in &mut result.groups {
            let members = root
                .descendants()
                .filter(|n| g.members.contains(&n.range().start) && is_move(*n))
                .collect::<Vec<_>>();
            if !members.iter().any(|n| n.has_tag_name((W, "moveFrom")))
                || !members.iter().any(|n| n.has_tag_name((W, "moveTo")))
            {
                g.error =
                    Some("The named move is missing its source or destination content".into());
            }
        }
        result
    }
    pub fn group(&self, n: Node) -> Result<&Group, String> {
        if let Some(error) = self.errors.get(&n.range().start) {
            return Err(error.clone());
        }
        let g = self
            .member_group
            .get(&n.range().start)
            .map(|i| &self.groups[*i])
            .ok_or("The move has no matched source and destination")?;
        if let Some(error) = &g.error {
            return Err(error.clone());
        }
        Ok(g)
    }
    /// Existing valid moves cannot be split accidentally by a containing edit.
    pub fn valid_groups(&self) -> impl Iterator<Item = &Group> {
        self.groups.iter().filter(|g| g.error.is_none())
    }
    pub fn check_after(&self, before: Node, after: Node) -> Result<(), String> {
        let next = Self::read(after);
        for group in self.valid_groups() {
            let matching = next
                .groups
                .iter()
                .find(|g| g.name == group.name && g.story == group.story);
            if let Some(g) = matching {
                if g.error.is_some() || g.members.len() != group.members.len() {
                    return Err("This edit would separate a tracked move from its linked location. Review the move first. Nothing changed.".into());
                }
            } else {
                let identities = before
                    .descendants()
                    .filter(|n| group.members.contains(&n.range().start) && is_move(*n))
                    .map(|n| (n.tag_name().name(), n.attribute((W, "id"))))
                    .collect::<Vec<_>>();
                if after.descendants().any(|n| {
                    is_move(n)
                        && identities.contains(&(n.tag_name().name(), n.attribute((W, "id"))))
                }) {
                    return Err("A moved revision lost its named range. Nothing changed.".into());
                }
            }
        }
        Ok(())
    }
}
