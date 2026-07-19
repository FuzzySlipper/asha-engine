use super::*;

pub(super) fn append_read_selector_identity(bytes: &mut Vec<u8>, selector: &GameplayReadSelector) {
    match selector {
        GameplayReadSelector::EventIdentity { binding } => {
            append_identity_text(bytes, "eventIdentity");
            append_event_binding_identity(bytes, binding);
        }
        GameplayReadSelector::Capability {
            binding,
            capability,
        } => {
            append_identity_text(bytes, "capability");
            append_event_binding_identity(bytes, binding);
            let kind = match capability {
                GameplayCapabilityReadKind::Lifecycle => "lifecycle",
                GameplayCapabilityReadKind::Transform => "transform",
                GameplayCapabilityReadKind::Collision => "collision",
                GameplayCapabilityReadKind::Controller => "controller",
            };
            append_identity_text(bytes, kind);
        }
        GameplayReadSelector::Related {
            binding,
            relationship,
        } => {
            append_identity_text(bytes, "related");
            append_event_binding_identity(bytes, binding);
            let kind = match relationship {
                GameplayRelationshipReadKind::TransformParent => "transformParent",
                GameplayRelationshipReadKind::Containment => "containment",
                GameplayRelationshipReadKind::SourceAncestry => "sourceAncestry",
            };
            append_identity_text(bytes, kind);
        }
        GameplayReadSelector::PrefabPart {
            instance,
            reference,
        } => {
            append_identity_text(bytes, "prefabPart");
            append_identity_u64(bytes, instance.raw());
            append_identity_u64(bytes, reference.prefab.raw());
            append_identity_text(bytes, &reference.role);
        }
        GameplayReadSelector::Tags {
            required_tags,
            max_items,
        } => {
            append_identity_text(bytes, "tags");
            append_identity_u64(bytes, required_tags.len() as u64);
            for tag in required_tags {
                append_identity_u64(bytes, tag.raw());
            }
            append_identity_u64(bytes, u64::from(*max_items));
        }
        GameplayReadSelector::Scope { scope, max_items } => {
            append_identity_text(bytes, "scope");
            append_identity_text(bytes, scope);
            append_identity_u64(bytes, u64::from(*max_items));
        }
        GameplayReadSelector::ModuleNamed { scope } => {
            append_identity_text(bytes, "moduleNamed");
            append_module_scope_identity(bytes, scope);
        }
        GameplayReadSelector::OwnerQuery { query } => {
            append_identity_text(bytes, "ownerQuery");
            append_owner_query_identity(bytes, query);
        }
    }
}
