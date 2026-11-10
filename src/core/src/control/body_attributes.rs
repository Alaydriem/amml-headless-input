use hcl::Body;

// The attributes of an HCL body, without its blocks.
pub struct BodyAttributes;

impl BodyAttributes {
    // the attributes of a body as a body of their own
    pub fn of(body: &Body) -> Body {
        Body::builder().add_attributes(body.attributes().cloned()).build()
    }
}
