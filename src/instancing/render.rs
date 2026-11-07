//! Crate-owned instanced drawing through Bevy's public render APIs.
use super::{PatternGeometryMaterial, TiledBackgroundMaterial};
use bevy::{
    asset::AssetId,
    ecs::{
        query::ROQueryItem,
        system::{
            SystemParamItem,
            lifetimeless::{Read, SRes},
        },
    },
    math::FloatOrd,
    prelude::*,
    render::{
        ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{PipelineCache, SpecializedRenderPipelines},
        texture::GpuImage,
        view::ExtractedView,
    },
    ui_render::{
        DrawUiMaterial, ExtractedUiMaterialNodes, PreparedUiMaterial, RenderUiSystems,
        SetMatUiViewBindGroup, TransparentUi, UiCameraView, UiMaterialBatch, UiMaterialMeta,
        UiMaterialPipeline, extract_ui_material_nodes, init_ui_material_pipeline,
        prepare_uimaterial_nodes, queue_ui_material_nodes, ui_material::UiMaterialKey,
    },
};

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<PatternGeometryMaterial>()
        .add_plugins(RenderAssetPlugin::<PreparedPattern, GpuImage>::default());
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .add_render_command::<TransparentUi, DrawPattern>()
            .init_resource::<ExtractedUiMaterialNodes<PatternGeometryMaterial>>()
            .init_gpu_resource::<UiMaterialMeta<PatternGeometryMaterial>>()
            .init_gpu_resource::<SpecializedRenderPipelines<UiMaterialPipeline<PatternGeometryMaterial>>>()
            .add_systems(RenderStartup, init_ui_material_pipeline::<PatternGeometryMaterial>)
            .add_systems(ExtractSchedule,
                extract_ui_material_nodes::<PatternGeometryMaterial>.in_set(RenderUiSystems::ExtractBackgrounds))
            .add_systems(Render, (
                queue.in_set(RenderSystems::Queue).after(queue_ui_material_nodes::<TiledBackgroundMaterial>),
                prepare_uimaterial_nodes::<PatternGeometryMaterial>.in_set(RenderSystems::PrepareBindGroups),
            ));
    }
}

// Keep the instance count with the prepared GPU material, not with the latest
// main-world asset: uploads may be delayed while an image is unavailable.
pub(super) struct PreparedPattern {
    material: PreparedUiMaterial<PatternGeometryMaterial>,
    instances: u32,
}

impl RenderAsset for PreparedPattern {
    type SourceAsset = PatternGeometryMaterial;
    type Param = <PreparedUiMaterial<PatternGeometryMaterial> as RenderAsset>::Param;

    fn prepare_asset(
        material: Self::SourceAsset,
        id: AssetId<Self::SourceAsset>,
        params: &mut SystemParamItem<Self::Param>,
        previous: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let instances = material.geometry.grid.element_product();
        PreparedUiMaterial::prepare_asset(material, id, params, previous.map(|p| &p.material)).map(
            |material| Self {
                material,
                instances,
            },
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "render resources are independently scheduled system parameters"
)]
fn queue(
    nodes: Res<ExtractedUiMaterialNodes<PatternGeometryMaterial>>,
    functions: Res<DrawFunctions<TransparentUi>>,
    pipeline: Res<UiMaterialPipeline<PatternGeometryMaterial>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<PatternGeometryMaterial>>>,
    cache: Res<PipelineCache>,
    materials: Res<RenderAssets<PreparedPattern>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    ui_views: Query<&UiCameraView>,
    views: Query<&ExtractedView>,
) {
    let draw_function = functions.read().id::<DrawPattern>();
    let reference_draw = functions
        .read()
        .id::<DrawUiMaterial<TiledBackgroundMaterial>>();
    for (index, node) in nodes.uinodes.iter().enumerate() {
        let Some(material) = materials.get(node.material) else {
            continue;
        };
        let Ok(ui_view) = ui_views.get(node.extracted_camera_entity) else {
            continue;
        };
        let Ok(view) = views.get(ui_view.0) else {
            continue;
        };
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(reference) = phase
            .items
            .values()
            .find(|item| item.draw_function == reference_draw && item.entity.1 == node.main_entity)
        else {
            continue;
        };
        // A newly queued pipeline has no indexable descriptor yet. Preserve the
        // reference draw until its pipeline has been processed and compiled.
        if cache.get_render_pipeline(reference.pipeline).is_none() {
            continue;
        }
        // Match the actual reference pass, including hosts that composite UI
        // through a different color format from their camera's main texture.
        let target_format = cache
            .get_render_pipeline_descriptor(reference.pipeline)
            .fragment
            .as_ref()
            .expect("UI material has a fragment shader")
            .targets[0]
            .as_ref()
            .expect("UI material has a color target")
            .format;
        let pipeline = pipelines.specialize(
            &cache,
            &pipeline,
            UiMaterialKey {
                target_format,
                bind_group_data: material.material.key,
            },
        );
        // Retain the original queued draw while the replacement material or
        // pipeline is loading. Swap only when this frame can actually render it.
        if cache.get_render_pipeline(pipeline).is_none() {
            continue;
        }
        let (render_entity, main_entity) = reference.entity;
        phase.remove(render_entity, main_entity);
        phase.add_transient(TransparentUi {
            draw_function,
            pipeline,
            entity: (node.render_entity, node.main_entity),
            sort_key: FloatOrd(node.stack_index as f32 + PatternGeometryMaterial::stack_z_offset()),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            index,
            indexed: false,
        });
    }
}

pub(super) type DrawPattern = (
    SetItemPipeline,
    SetMatUiViewBindGroup<PatternGeometryMaterial, 0>,
    DrawInstances,
);

pub(super) struct DrawInstances;
impl<P: PhaseItem> RenderCommand<P> for DrawInstances {
    type Param = SRes<RenderAssets<PreparedPattern>>;
    type ViewQuery = ();
    type ItemQuery = Read<UiMaterialBatch<PatternGeometryMaterial>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<ROQueryItem<'_, '_, Self::ItemQuery>>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials.into_inner().get(batch.material) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(1, &material.material.bind_group, &[]);
        pass.draw(0..6, 0..material.instances);
        RenderCommandResult::Success
    }
}
