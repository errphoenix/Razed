use rendrs::graphics::material::{MaterialGroup, MaterialLocationRegistry};

use crate::assets::TextureRegistry;

#[derive(Debug)]
pub struct Groups {
    pub dev: MaterialGroup,
}

#[derive(Debug, Default)]
pub struct Materials {
    location_registry: MaterialLocationRegistry,
    pub groups: Option<Groups>,
}
impl Materials {
    pub fn empty() -> Self {
        Self {
            location_registry: MaterialLocationRegistry::new(),
            groups: None,
        }
    }

    pub fn initialize(&mut self, texture_registry: &mut TextureRegistry) {
        self.groups = Some(Groups {
            dev: material_group_dev(0, texture_registry, &mut self.location_registry),
        })
    }

    pub const fn groups_opt(&self) -> Option<&Groups> {
        self.groups.as_ref()
    }

    pub fn groups(&self) -> &Groups {
        self.groups.as_ref().unwrap()
    }

    pub const fn locations(&self) -> &MaterialLocationRegistry {
        &self.location_registry
    }

    pub const fn locations_mut(&mut self) -> &mut MaterialLocationRegistry {
        &mut self.location_registry
    }
}

rendrs::material_groups! {
    group Dev {
        pages: 32;
        size: 1024;

        entry(materials::MATERIAL_DEV_NAME_BRICKS103) {
            diffuse      = asset(*materials::MATEX_DEV_ID_BRICKS103_DIFFUSE);
            normal       = asset(*materials::MATEX_DEV_ID_BRICKS103_NORMAL);
            occlusion    = asset(*materials::MATEX_DEV_ID_BRICKS103_OCCLUSION);
            roughness    = asset(*materials::MATEX_DEV_ID_BRICKS103_ROUGHNESS);
            displacement = asset(*materials::MATEX_DEV_ID_BRICKS103_DISPLACEMENT);
        };
        entry(materials::MATERIAL_DEV_NAME_CONCRETE012) {
            diffuse      = asset(*materials::MATEX_DEV_ID_CONCRETE012_DIFFUSE);
            normal       = asset(*materials::MATEX_DEV_ID_CONCRETE012_NORMAL);
            roughness    = asset(*materials::MATEX_DEV_ID_CONCRETE012_ROUGHNESS);
            displacement = asset(*materials::MATEX_DEV_ID_CONCRETE012_DISPLACEMENT);
        };
        entry(materials::MATERIAL_DEV_NAME_ROAD015C) {
            diffuse      = asset(*materials::MATEX_DEV_ID_ROAD015C_DIFFUSE);
            normal       = asset(*materials::MATEX_DEV_ID_ROAD015C_NORMAL);
            occlusion    = asset(*materials::MATEX_DEV_ID_ROAD015C_OCCLUSION);
            roughness    = asset(*materials::MATEX_DEV_ID_ROAD015C_ROUGHNESS);
            displacement = asset(*materials::MATEX_DEV_ID_ROAD015C_DISPLACEMENT);
        };
        entry(materials::MATERIAL_DEV_NAME_METAL048C) {
            diffuse      = asset(*materials::MATEX_DEV_ID_METAL048C_DIFFUSE);
            normal       = asset(*materials::MATEX_DEV_ID_METAL048C_NORMAL);
            metallic     = asset(*materials::MATEX_DEV_ID_METAL048C_METALLIC);
            roughness    = asset(*materials::MATEX_DEV_ID_METAL048C_ROUGHNESS);
            displacement = asset(*materials::MATEX_DEV_ID_METAL048C_DISPLACEMENT);
        };
    }
}

// should probably load from a file
pub mod materials {
    macro_rules! define {
        (
            $macrogroup:ident . $name:ident => $($comp:ident$(,)?)+
        ) => {
            paste::paste! {
                #[allow(unused)]
                pub const [< MATERIAL_ $macrogroup:upper _NAME_ $name:upper >]: &'static str =
                    concat!(
                        "__", stringify!($macrogroup:lower),
                        ".", stringify!($name:lower)
                    );

                $(
                    #[allow(unused)]
                    pub const [< MATEX_ $macrogroup:upper _NAME_ $name:upper _ $comp:upper >]: &'static str =
                        concat!(
                            "__", stringify!($macrogroup:lower), ".",
                            stringify!($name:lower),
                            ".", stringify!($comp:lower)
                        );
                )+

                ethel::hashet! {
                    pub const [< MATERIAL_ $macrogroup:upper _ID_ $name:upper >] = [< MATERIAL_ $macrogroup:upper _NAME_ $name:upper >];

                    $(
                        pub const [< MATEX_ $macrogroup:upper _ID_ $name:upper _ $comp:upper >] = [< MATEX_ $macrogroup:upper _NAME_ $name:upper _ $comp:upper >];
                    )+
                }
            }
        };

        (
            $( $macrogroup:ident . $name:ident => $($comp:ident$(,)?)+;)+
        ) => {
            $(define!($macrogroup . $name => $($comp,)+);)+
        };
    }

    define! {
        dev.bricks103 => diffuse, normal, occlusion, roughness, displacement;
        dev.concrete012 => diffuse, normal, roughness, displacement;
        dev.road015c => diffuse, normal, occlusion, roughness, displacement;
        dev.metal048c => diffuse, normal, metallic, roughness, displacement;
    }
}
