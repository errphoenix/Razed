use ethel::assets::{AssetRegistry, Handle, HasMetadata, RawTexture, TextureMetadata};
use janus::texture::{ImageFormat, ImageType, MipLevels, Tex, Texture};
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
        entry(materials::MATERIAL_DEV_NAME_METAL063) {
            diffuse      = asset(*materials::MATEX_DEV_ID_METAL063_DIFFUSE);
            normal       = asset(*materials::MATEX_DEV_ID_METAL063_NORMAL);
            metallic     = asset(*materials::MATEX_DEV_ID_METAL063_METALLIC);
            roughness    = asset(*materials::MATEX_DEV_ID_METAL063_ROUGHNESS);
            displacement = asset(*materials::MATEX_DEV_ID_METAL063_DISPLACEMENT);
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
}

pub fn pack_cubemap(
    registry: &mut AssetRegistry<RawTexture, TextureMetadata>,
    common_id: &str,
) -> Texture {
    let id_negx = janus::hash_string(&format!("{common_id}.negx"));
    let id_negy = janus::hash_string(&format!("{common_id}.negy"));
    let id_negz = janus::hash_string(&format!("{common_id}.negz"));
    let id_posx = janus::hash_string(&format!("{common_id}.posx"));
    let id_posy = janus::hash_string(&format!("{common_id}.posy"));
    let id_posz = janus::hash_string(&format!("{common_id}.posz"));

    let negx = registry.unregister(id_negx).unwrap();
    let negy = registry.unregister(id_negy).unwrap();
    let negz = registry.unregister(id_negz).unwrap();
    let posx = registry.unregister(id_posx).unwrap();
    let posy = registry.unregister(id_posy).unwrap();
    let posz = registry.unregister(id_posz).unwrap();

    pack_cubemap_faces(negx, negy, negz, posx, posy, posz)
}

type TextureAssetHandle = Handle<RawTexture, TextureMetadata>;

pub fn pack_cubemap_faces(
    negx: TextureAssetHandle,
    negy: TextureAssetHandle,
    negz: TextureAssetHandle,
    posx: TextureAssetHandle,
    posy: TextureAssetHandle,
    posz: TextureAssetHandle,
) -> Texture {
    {
        let negx = negx.metadata();
        let negy = negy.metadata();
        let negz = negz.metadata();
        let posx = posx.metadata();
        let posy = posy.metadata();
        let posz = posz.metadata();
        assert!(
            negx.size == negy.size
                && negy.size == negz.size
                && negz.size == posx.size
                && posx.size == posy.size
                && posy.size == posz.size
        );
    }

    let (w, h) = negx
        .metadata()
        .size
        .expect("texture asset and its metadata must be properly initialised");
    let cubemap = Texture::new_cubemap(
        w as i32,
        h as i32,
        MipLevels::default(),
        ImageType::Bits8,
        ImageFormat::Rgb,
    );

    let faces = [
        negx.raw_resource().unwrap().0.as_bytes(),
        negy.raw_resource().unwrap().0.as_bytes(),
        negz.raw_resource().unwrap().0.as_bytes(),
        posx.raw_resource().unwrap().0.as_bytes(),
        posy.raw_resource().unwrap().0.as_bytes(),
        posz.raw_resource().unwrap().0.as_bytes(),
    ];

    for i in 0..6 {
        cubemap.upload_layer(0, 0, 0, i, w as i32, h as i32, faces[i as usize]);
    }

    cubemap
}
