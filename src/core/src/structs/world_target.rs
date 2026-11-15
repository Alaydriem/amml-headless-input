// The world a join_world step opens. `name` is what the user gave; `level_id` is the folder id the game opens it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTarget {
    pub name: String,
    pub level_id: String,
}
