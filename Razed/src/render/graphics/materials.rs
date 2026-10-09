use ethel::render::buffer::SingleBuffer;
use janus::StringMap;
use rendrs::graphics::material::{MaterialGroup, MaterialLocation, MaterialLocationRegistry};

use crate::assets::TextureRegistry;

pub type MaterialStorage = SingleBuffer<MaterialLocation>;

pub const MATERIAL_CAPACITY: usize = 2048;

#[derive(Debug)]
pub struct Groups {
    pub dev: MaterialGroup,
}

#[derive(Debug, Default)]
pub struct Materials {
    list: Vec<MaterialLocation>,
    map_registry: StringMap<usize>,
    location_registry: MaterialLocationRegistry,
    pub groups: Option<Groups>,
    dirty: bool,
}
#[allow(unused)]
impl Materials {
    pub fn empty() -> Self {
        Self {
            list: Vec::new(),
            map_registry: StringMap::default(),
            location_registry: MaterialLocationRegistry::new(),
            groups: None,
            dirty: false,
        }
    }

    pub fn initialize(&mut self, texture_registry: &mut TextureRegistry) {
        self.dirty = true;

        self.groups = Some(Groups {
            dev: material_group_dev(0, texture_registry, &mut self.location_registry),
        });

        self.location_registry
            .inner_map()
            .iter()
            .for_each(|(&id, &loc)| {
                let index = self.list.len();
                self.list.push(loc);
                self.map_registry.insert(id, index);
            });
    }

    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub const fn set_dirty(&mut self) {
        self.dirty = true;
    }

    pub unsafe fn upload_to_buffer(&mut self, buffer: &MaterialStorage) {
        if self.dirty {
            self.dirty = false;
            unsafe {
                buffer.blit(self.list(), 0);
            }
        }
    }

    pub fn list(&self) -> &[MaterialLocation] {
        &self.list
    }

    pub const fn list_mut(&mut self) -> &mut Vec<MaterialLocation> {
        &mut self.list
    }

    pub const fn map(&self) -> &StringMap<usize> {
        &self.map_registry
    }

    pub const fn map_mut(&mut self) -> &mut StringMap<usize> {
        &mut self.map_registry
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

        entry(MATERIAL_DEV_NAME_BRICKS103) {
            diffuse      = asset(*MATEX_DEV_ID_BRICKS103_DIFFUSE);
            normal       = asset(*MATEX_DEV_ID_BRICKS103_NORMAL);
            occlusion    = asset(*MATEX_DEV_ID_BRICKS103_OCCLUSION);
            roughness    = asset(*MATEX_DEV_ID_BRICKS103_ROUGHNESS);
            displacement = asset(*MATEX_DEV_ID_BRICKS103_DISPLACEMENT);
        };
        entry(MATERIAL_DEV_NAME_CONCRETE012) {
            diffuse      = asset(*MATEX_DEV_ID_CONCRETE012_DIFFUSE);
            normal       = asset(*MATEX_DEV_ID_CONCRETE012_NORMAL);
            roughness    = asset(*MATEX_DEV_ID_CONCRETE012_ROUGHNESS);
            displacement = asset(*MATEX_DEV_ID_CONCRETE012_DISPLACEMENT);
        };
        entry(MATERIAL_DEV_NAME_ROAD015C) {
            diffuse      = asset(*MATEX_DEV_ID_ROAD015C_DIFFUSE);
            normal       = asset(*MATEX_DEV_ID_ROAD015C_NORMAL);
            occlusion    = asset(*MATEX_DEV_ID_ROAD015C_OCCLUSION);
            roughness    = asset(*MATEX_DEV_ID_ROAD015C_ROUGHNESS);
            displacement = asset(*MATEX_DEV_ID_ROAD015C_DISPLACEMENT);
        };
        entry(MATERIAL_DEV_NAME_METAL048C) {
            diffuse      = asset(*MATEX_DEV_ID_METAL048C_DIFFUSE);
            normal       = asset(*MATEX_DEV_ID_METAL048C_NORMAL);
            metallic     = asset(*MATEX_DEV_ID_METAL048C_METALLIC);
            roughness    = asset(*MATEX_DEV_ID_METAL048C_ROUGHNESS);
            displacement = asset(*MATEX_DEV_ID_METAL048C_DISPLACEMENT);
        };
        entry(MATERIAL_DEV_NAME_METAL063) {
            diffuse      = asset(*MATEX_DEV_ID_METAL063_DIFFUSE);
            normal       = asset(*MATEX_DEV_ID_METAL063_NORMAL);
            metallic     = asset(*MATEX_DEV_ID_METAL063_METALLIC);
            roughness    = asset(*MATEX_DEV_ID_METAL063_ROUGHNESS);
            displacement = asset(*MATEX_DEV_ID_METAL063_DISPLACEMENT);
        };
    }
}

// should probably load from a file
macro_rules! define {
    (
        $macrogroup:ident . $name:ident => $($comp:ident$(,)?)+
    ) => {
        paste::paste! {
            #[allow(unused)]
            pub const [< MATERIAL_ $macrogroup:upper _NAME_ $name:upper >]: &'static str =
                concat!(
                    "__", stringify!([< $macrogroup:lower >]),
                    ".", stringify!([< $name:lower >])
                );

            $(
                #[allow(unused)]
                pub const [< MATEX_ $macrogroup:upper _NAME_ $name:upper _ $comp:upper >]: &'static str =
                    concat!(
                        "__", stringify!([< $macrogroup:lower >]), ".",
                        stringify!([< $name:lower >]),
                        ".", stringify!([< $comp:lower >])
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
    dev.metal063 => diffuse, normal, metallic, roughness, displacement;
}
