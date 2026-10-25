//! Geometry recorded from actual menu drawing, enabled only for layout QA.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Text,
    Button,
    Panel,
}
#[derive(Debug)]
pub struct Item {
    pub kind: Kind,
    pub label: String,
    pub rect: [f32; 4],
}
impl Item {
    fn overlaps(&self, b: &Self) -> bool {
        let [x, y, w, h] = self.rect;
        let [bx, by, bw, bh] = b.rect;
        (x + w).min(bx + bw) - x.max(bx) > 0.5 && (y + h).min(by + bh) - y.max(by) > 0.5
    }
    fn contains(&self, b: &Self) -> bool {
        let [x, y, w, h] = self.rect;
        let [bx, by, bw, bh] = b.rect;
        bx >= x - 0.5 && by >= y - 0.5 && bx + bw <= x + w + 0.5 && by + bh <= y + h + 0.5
    }
}
#[derive(Default)]
pub struct Audit {
    pub items: Vec<Item>,
}
impl Audit {
    pub fn add(&mut self, kind: Kind, label: &str, rect: [f32; 4]) {
        self.items.push(Item {
            kind,
            label: label.into(),
            rect,
        });
    }
    pub fn issues(&self, w: f32, h: f32) -> Vec<String> {
        let window = Item {
            kind: Kind::Panel,
            label: "window".into(),
            rect: [0.0, 0.0, w, h],
        };
        let mut issues = Vec::new();
        for (i, a) in self.items.iter().enumerate() {
            if a.rect.iter().any(|v| !v.is_finite()) || !window.contains(a) {
                issues.push(format!(
                    "outside window: {:?} {} {:?}",
                    a.kind, a.label, a.rect
                ));
            }
            for b in &self.items[i + 1..] {
                if !a.overlaps(b) {
                    continue;
                }
                let conflict = match (a.kind, b.kind) {
                    (Kind::Text, Kind::Text) | (Kind::Button, Kind::Button) => true,
                    (Kind::Text, Kind::Button) => true,
                    (Kind::Button, Kind::Text) => !a.contains(b),
                    (Kind::Panel, Kind::Text | Kind::Button) => !a.contains(b),
                    (Kind::Text | Kind::Button, Kind::Panel) => true,
                    _ => false,
                };
                if conflict {
                    issues.push(format!(
                        "overlap: {:?} {} {:?}, {:?} {} {:?}",
                        a.kind, a.label, a.rect, b.kind, b.label, b.rect
                    ));
                }
            }
        }
        issues
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn later_briefing_panel_cannot_cover_a_button_or_its_label() {
        let mut a = Audit::default();
        a.add(Kind::Button, "QUIT", [941.0, 578.0, 117.0, 45.0]);
        a.add(Kind::Text, "QUIT", [965.0, 590.0, 69.0, 21.0]);
        a.add(Kind::Panel, "MISSION DATA", [1046.0, 153.0, 738.0, 594.0]);
        assert_eq!(a.issues(1845.0, 859.0).len(), 1);
    }
    #[test]
    fn labels_belong_inside_their_control_and_other_text_stays_clear() {
        let mut a = Audit::default();
        a.add(Kind::Button, "mode", [10.0, 10.0, 100.0, 30.0]);
        a.add(Kind::Text, "mode", [15.0, 15.0, 50.0, 10.0]);
        assert!(a.issues(640.0, 360.0).is_empty());
        a.add(Kind::Text, "description", [108.0, 15.0, 60.0, 10.0]);
        assert!(!a.issues(640.0, 360.0).is_empty());
    }
}
