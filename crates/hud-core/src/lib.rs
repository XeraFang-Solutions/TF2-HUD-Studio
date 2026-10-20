//! HUD Studio core: everything that understands a TF2 HUD as a whole — file layering (project, tf/, VPKs),
//! #base resolution, scheme, images, fonts, animations, problems and project operations.
pub mod anim;
pub mod fontmeta;
pub mod images;
pub mod lint;
pub mod project;
pub mod resolve;
pub mod scheme;
pub mod vfs;
pub mod vpk;
pub mod vtf;
