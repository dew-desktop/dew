--!strict
-- GENERATED FILE. Do not hand-edit -- regenerate with:
--     cargo run --manifest-path host/Cargo.toml --bin generate-datamodel-types
-- and commit the result. CI's `Check the generated DataModel editor types are
-- current` step runs the same binary with `--check`, which regenerates into
-- memory and diffs against this file without writing, the same discipline
-- `docs/datamodel_scope.md` already asks for in its own header comment.
--
-- Owns the entire DataModel class and enum hierarchy this host accepts,
-- including `host/src/datamodel/extensions.rs`'s Dew-original rows
-- (`InputActionLabel.InputAction`, `GuiObject.BlendingMode`/`Enum.BlendMode`)
-- merged in at generation time. It does NOT declare value types (`Vector3`,
-- `CFrame`, `Color3`, `UDim2`, `Content`, `Font`, `BrickColor`,
-- `NumberSequence`, `ColorSequence`, `PhysicalProperties`, and the rest) or
-- the `Enum`/`EnumItem` base types -- those are real global types
-- `luau-lsp.types.roblox` already provides.
--
-- KNOWN GAP: PROPERTIES ONLY, NO METHODS OR EVENTS. Every class below has
-- accurate properties and nothing else -- `rbx_reflection_database` (the
-- only source this generator reads) carries no method or event signatures
-- for any class, ever. Because this file's classes replace Roblox's own
-- same-named declarations outright rather than merging, a script calling
-- `instance:Clone()`, connecting `RunService.Heartbeat`, or calling
-- `GetPropertyChangedSignal` now gets a real editor error it did not get
-- before this file was wired in. This is a known, accepted tradeoff -- see
-- `generate_datamodel_types.rs`'s own header for why two narrower designs
-- were tried and rejected. `scripts/fetch_api_surface.luau`'s vendored
-- ~40-class method/event subset is the incremental path to closing this gap
-- further; extend that over time rather than reopening this generator's
-- scope to try to solve it all at once.

-- ── Classes ─────────────────────────────────────────────────────────────

declare extern type Object with
	ClassName: string
	className: string
end

declare extern type Instance extends Object with
	Archivable: boolean
	Name: string
	Parent: Instance?
	RobloxLocked: boolean
	Sandboxed: boolean
	archivable: boolean
end

declare extern type Accoutrement extends Instance with
	AttachmentForward: Vector3
	AttachmentPoint: CFrame
	AttachmentPos: Vector3
	AttachmentRight: Vector3
	AttachmentUp: Vector3
end

declare extern type Accessory extends Accoutrement with
	AccessoryType: EnumAccessoryType
end

declare extern type AccessoryDescription extends Instance with
	AccessoryType: EnumAccessoryType
	AssetId: number
	Instance: Instance?
	IsLayered: boolean
	Order: number
	Position: Vector3
	Puffiness: number
	Rotation: Vector3
	Scale: Vector3
end

declare extern type AccountService extends Instance with
end

declare extern type AchievementService extends Instance with
end

declare extern type ActivityHistoryEventService extends Instance with
end

declare extern type PVInstance extends Instance with
end

declare extern type Model extends PVInstance with
	LevelOfDetail: EnumModelLevelOfDetail
	ModelStreamingMode: EnumModelStreamingMode
	PrimaryPart: Instance?
	Scale: number
	WorldPivot: CFrame
	WorldPivotData: CFrame?
end

declare extern type Actor extends Model with
end

declare extern type GuiBase extends Instance with
end

declare extern type GuiBase2d extends GuiBase with
	AbsolutePosition: Vector2
	AbsoluteRotation: number
	AbsoluteSize: Vector2
	AutoLocalize: boolean
	Localize: boolean
	RootLocalizationTable: Instance?
	SelectionBehaviorDown: EnumSelectionBehavior
	SelectionBehaviorLeft: EnumSelectionBehavior
	SelectionBehaviorRight: EnumSelectionBehavior
	SelectionBehaviorUp: EnumSelectionBehavior
	SelectionGroup: boolean
end

declare extern type LayerCollector extends GuiBase2d with
	Enabled: boolean
	ResetOnSpawn: boolean
	ZIndexBehavior: EnumZIndexBehavior
end

declare extern type SurfaceGuiBase extends LayerCollector with
	Active: boolean
	Adornee: Instance?
	Face: EnumNormalId
end

declare extern type AdGui extends SurfaceGuiBase with
	AdShape: EnumAdShape
	EnableVideoAds: boolean
	FallbackImage: string
	FallbackImageContent: Content
	Status: EnumAdUnitStatus
end

declare extern type AdPortal extends Instance with
	Status: EnumAdUnitStatus
end

declare extern type AdService extends Instance with
end

declare extern type AdvancedDragger extends Instance with
end

declare extern type ControllerBase extends Instance with
	Active: boolean
	BalanceRigidityEnabled: boolean
	MoveSpeedFactor: number
end

declare extern type AirController extends ControllerBase with
	BalanceMaxTorque: number
	BalanceSpeed: number
	LinearImpulse: Vector3
	MaintainAngularMomentum: boolean
	MaintainLinearMomentum: boolean
	MoveMaxForce: number
	TurnMaxTorque: number
	TurnSpeedFactor: number
end

declare extern type Constraint extends Instance with
	Active: boolean
	Attachment0: Instance?
	Attachment1: Instance?
	Color: BrickColor
	Enabled: boolean
	Visible: boolean
end

declare extern type AlignOrientation extends Constraint with
	AlignType: EnumAlignType
	CFrame: CFrame
	LookAtPosition: Vector3
	MaxAngularVelocity: number
	MaxTorque: number
	Mode: EnumOrientationAlignmentMode
	PrimaryAxis: Vector3
	PrimaryAxisOnly: boolean
	ReactionTorqueEnabled: boolean
	Responsiveness: number
	RigidityEnabled: boolean
	SecondaryAxis: Vector3
end

declare extern type AlignPosition extends Constraint with
	ApplyAtCenterOfMass: boolean
	ForceLimitMode: EnumForceLimitMode
	ForceRelativeTo: EnumActuatorRelativeTo
	MaxAxesForce: Vector3
	MaxForce: number
	MaxVelocity: number
	Mode: EnumPositionAlignmentMode
	Position: Vector3
	ReactionForceEnabled: boolean
	Responsiveness: number
	RigidityEnabled: boolean
end

declare extern type AnalyticsService extends Instance with
end

declare extern type AngularVelocity extends Constraint with
	AngularVelocity: Vector3
	MaxTorque: number
	ReactionTorqueEnabled: boolean
	RelativeTo: EnumActuatorRelativeTo
end

declare extern type Animation extends Instance with
	AnimationContent: Content
	AnimationId: string
end

declare extern type AnimationClip extends Instance with
	Length: number
	Loop: boolean
	Priority: EnumAnimationPriority
end

declare extern type AnimationClipProvider extends Instance with
end

declare extern type AnimationConstraint extends Constraint with
	AngularDamping: number
	AngularStrength: number
	C0: CFrame
	C1: CFrame
	IsKinematic: boolean
	LinearDamping: number
	LinearStrength: number
	MaxForce: number
	MaxTorque: number
	Part0: Instance?
	Part1: Instance?
	Transform: CFrame
end

declare extern type AnimationController extends Instance with
end

declare extern type AnimationFromVideoCreatorService extends Instance with
end

declare extern type AnimationFromVideoCreatorStudioService extends Instance with
end

declare extern type AnimationGraphDefinition extends AnimationClip with
end

declare extern type BaseImportData extends Instance with
	Id: string
	ImportName: string
	ShouldImport: boolean
end

declare extern type AnimationImportData extends BaseImportData with
end

declare extern type AnimationNode extends Object with
end

declare extern type AnimationNodeDefinition extends Instance with
	NodeType: EnumAnimationNodeType
end

declare extern type AnimationRigData extends Instance with
end

declare extern type AnimationStreamTrack extends Instance with
	Animation: Instance?
	FACSDataLod: EnumFACSDataLod
	IsPlaying: boolean
	Priority: EnumAnimationPriority
	WeightCurrent: number
	WeightTarget: number
end

declare extern type AnimationTrack extends Instance with
	Animation: Instance?
	IsPlaying: boolean
	Length: number
	Looped: boolean
	Priority: EnumAnimationPriority
	Speed: number
	TimePosition: number
	WeightCurrent: number
	WeightTarget: number
end

declare extern type Animator extends Instance with
	EvaluationThrottled: boolean
	PreferLodEnabled: boolean
	RootMotion: CFrame
	RootMotionWeight: number
end

declare extern type Annotation extends Instance with
end

declare extern type AnnotationsService extends Instance with
end

declare extern type AppAgeSignalsService extends Instance with
end

declare extern type AppLifecycleObserverService extends Instance with
end

declare extern type AppRatingPromptService extends Instance with
end

declare extern type LocalStorageService extends Instance with
end

declare extern type AppStorageService extends LocalStorageService with
end

declare extern type AppUpdateService extends Instance with
end

declare extern type GuiBase3d extends GuiBase with
	Color: BrickColor
	Color3: Color3
	Transparency: number
	Visible: boolean
end

declare extern type PartAdornment extends GuiBase3d with
	Adornee: Instance?
end

declare extern type HandlesBase extends PartAdornment with
end

declare extern type ArcHandles extends HandlesBase with
	Axes: Axes
end

declare extern type AssetCounterService extends Instance with
end

declare extern type AssetDeliveryProxy extends Instance with
	Interface: string
	Port: number
	StartServer: boolean
end

declare extern type AssetImportService extends Instance with
end

declare extern type ImportSession extends Instance with
end

declare extern type AssetImportSession extends ImportSession with
end

declare extern type AssetManagerService extends Instance with
end

declare extern type AssetPatchSettings extends Instance with
	ContentId: string
	OutputPath: string
	PatchId: string
end

declare extern type AssetQualityService extends Instance with
end

declare extern type AssetService extends Instance with
end

declare extern type SoundEffect extends Instance with
	Enabled: boolean
	Priority: number
end

declare extern type CustomSoundEffect extends SoundEffect with
end

declare extern type AssetSoundEffect extends CustomSoundEffect with
end

declare extern type Atmosphere extends Instance with
	Color: Color3
	Decay: Color3
	Density: number
	Glare: number
	Haze: number
	Offset: number
end

declare extern type SensorBase extends Instance with
	UpdateType: EnumSensorUpdateType
end

declare extern type AtmosphereSensor extends SensorBase with
	AirDensity: number
	RelativeWindVelocity: Vector3
end

declare extern type Attachment extends Instance with
	Axis: Vector3
	CFrame: CFrame
	Orientation: Vector3
	Position: Vector3
	Rotation: Vector3
	SecondaryAxis: Vector3
	Visible: boolean
	WorldAxis: Vector3
	WorldCFrame: CFrame
	WorldOrientation: Vector3
	WorldPosition: Vector3
	WorldRotation: Vector3
	WorldSecondaryAxis: Vector3
end

declare extern type AudioAnalyzer extends Instance with
	PeakLevel: number
	RmsLevel: number
	SpectrumEnabled: boolean
	WindowSize: EnumAudioWindowSize
end

declare extern type AudioChannelMixer extends Instance with
	Layout: EnumAudioChannelLayout
end

declare extern type AudioChannelSplitter extends Instance with
	Layout: EnumAudioChannelLayout
end

declare extern type AudioChorus extends Instance with
	Bypass: boolean
	Depth: number
	Mix: number
	Rate: number
end

declare extern type AudioCompressor extends Instance with
	Attack: number
	Bypass: boolean
	MakeupGain: number
	Ratio: number
	Release: number
	Threshold: number
end

declare extern type AudioDeviceInput extends Instance with
	AccessType: EnumAccessModifierType
	Active: boolean
	Muted: boolean
	Player: Instance?
	Volume: number
end

declare extern type AudioDeviceOutput extends Instance with
	Player: Instance?
end

declare extern type AudioDistortion extends Instance with
	Bypass: boolean
	Level: number
end

declare extern type AudioEcho extends Instance with
	Bypass: boolean
	DelayTime: number
	DryLevel: number
	Feedback: number
	RampTime: number
	WetLevel: number
end

declare extern type AudioEmitter extends Instance with
	AcousticSimulationEnabled: boolean
	AudioInteractionGroup: string
	DiffractionEnabled: EnumSimulationMode
	OcclusionEnabled: EnumSimulationMode
	PositionInstance: Instance?
	PositionType: EnumEmitterPositionType
	ReverbEnabled: EnumSimulationMode
	SimulationFidelity: EnumAudioSimulationFidelity
end

declare extern type AudioEqualizer extends Instance with
	Bypass: boolean
	HighGain: number
	LowGain: number
	MidGain: number
	MidRange: NumberRange
end

declare extern type AudioFader extends Instance with
	Bypass: boolean
	Volume: number
end

declare extern type AudioFilter extends Instance with
	Bypass: boolean
	FilterType: EnumAudioFilterType
	Frequency: number
	Gain: number
	Q: number
end

declare extern type AudioFlanger extends Instance with
	Bypass: boolean
	Depth: number
	Mix: number
	Rate: number
end

declare extern type AudioFocusService extends Instance with
end

declare extern type AudioGate extends Instance with
	Attack: number
	Bypass: boolean
	Release: number
	Threshold: NumberRange
end

declare extern type AudioLimiter extends Instance with
	Bypass: boolean
	MaxLevel: number
	Release: number
end

declare extern type AudioListener extends Instance with
	AcousticSimulationEnabled: boolean
	AudioInteractionGroup: string
	DiffractionEnabled: EnumSimulationMode
	OcclusionEnabled: EnumSimulationMode
	PositionInstance: Instance?
	PositionType: EnumListenerPositionType
	ReverbEnabled: EnumSimulationMode
	SimulationFidelity: EnumAudioSimulationFidelity
end

declare extern type Pages extends Instance with
	IsFinished: boolean
end

declare extern type AudioPages extends Pages with
end

declare extern type AudioPitchShifter extends Instance with
	Bypass: boolean
	Pitch: number
	WindowSize: EnumAudioWindowSize
end

declare extern type AudioPlayer extends Instance with
	Asset: string
	AssetId: string
	AudioContent: Content
	AutoLoad: boolean
	AutoPlay: boolean
	IsPlaying: boolean
	IsReady: boolean
	LoopRegion: NumberRange
	Looping: boolean
	PlaybackRegion: NumberRange
	PlaybackSpeed: number
	TimeLength: number
	TimePosition: number
	Volume: number
end

declare extern type AudioRecorder extends Instance with
	IsRecording: boolean
	TimeLength: number
end

declare extern type AudioReverb extends Instance with
	Bypass: boolean
	DecayRatio: number
	DecayTime: number
	Density: number
	Diffusion: number
	DryLevel: number
	EarlyDelayTime: number
	HighCutFrequency: number
	LateDelayTime: number
	LowShelfFrequency: number
	LowShelfGain: number
	ReferenceFrequency: number
	WetLevel: number
end

declare extern type AudioSearchParams extends Instance with
	Album: string
	Artist: string
	AudioSubType: EnumAudioSubType
	AudioSubtype: EnumAudioSubType
	MaxDuration: number
	MinDuration: number
	SearchKeyword: string
	Tag: string
	Title: string
end

declare extern type AudioSpeechToText extends Instance with
	Enabled: boolean
	Text: string
	VoiceDetected: boolean
end

declare extern type AudioTextToSpeech extends Instance with
	IsLoaded: boolean
	IsPlaying: boolean
	Looping: boolean
	Pitch: number
	PlaybackSpeed: number
	Speed: number
	Text: string
	TimeLength: number
	TimePosition: number
	VoiceId: string
	Volume: number
end

declare extern type AudioTremolo extends Instance with
	Bypass: boolean
	Depth: number
	Duty: number
	Frequency: number
	Shape: number
	Skew: number
	Square: number
end

declare extern type LuaSourceContainer extends Instance with
end

declare extern type AuroraScript extends LuaSourceContainer with
	EnableCulling: boolean
	EnableLOD: boolean
	LODCriticality: number
	Priority: number
	Source: string
end

declare extern type AuroraScriptObject extends Instance with
	FrameId: number
	LODLevel: number
	PriorFrameInvoked: number
end

declare extern type AuroraScriptService extends Instance with
end

declare extern type AuroraService extends Instance with
	HashRoundingPoint: number
	IgnoreRotation: boolean
	LockStepIdOffset: boolean
	RollbackOffset: number
end

declare extern type AvatarAbilityRules extends Instance with
end

declare extern type AvatarAccessoryRules extends Instance with
end

declare extern type AvatarAnimationRules extends Instance with
end

declare extern type AvatarBodyRules extends Instance with
end

declare extern type AvatarChatService extends Instance with
end

declare extern type AvatarClothingRules extends Instance with
end

declare extern type AvatarCollisionRules extends Instance with
end

declare extern type AvatarCreationService extends Instance with
end

declare extern type AvatarEditorService extends Instance with
end

declare extern type AvatarImportService extends Instance with
end

declare extern type AvatarRules extends Instance with
end

declare extern type AvatarSettings extends Instance with
end

declare extern type Backpack extends Instance with
end

declare extern type BackpackItem extends Model with
	TextureContent: Content
	TextureId: string
end

declare extern type BadgeService extends Instance with
end

declare extern type BallSocketConstraint extends Constraint with
	LimitsEnabled: boolean
	MaxFrictionTorque: number
	Radius: number
	Restitution: number
	TwistLimitsEnabled: boolean
	TwistLowerAngle: number
	TwistUpperAngle: number
	UpperAngle: number
end

declare extern type BanHistoryPages extends Pages with
end

declare extern type BaseCoreGuiConfiguration extends Instance with
	Enabled: boolean
end

declare extern type BasePart extends PVInstance with
	Anchored: boolean
	AssemblyAngularVelocity: Vector3
	AssemblyCenterOfMass: Vector3
	AssemblyLinearVelocity: Vector3
	AssemblyMass: number
	AssemblyRootPart: Instance?
	AudioCanCollide: boolean
	BackParamA: number
	BackParamB: number
	BackSurface: EnumSurfaceType
	BackSurfaceInput: EnumInputType
	BottomParamA: number
	BottomParamB: number
	BottomSurface: EnumSurfaceType
	BottomSurfaceInput: EnumInputType
	BrickColor: BrickColor
	CFrame: CFrame
	CanCollide: boolean
	CanQuery: boolean
	CanTouch: boolean
	CastShadow: boolean
	CenterOfMass: Vector3
	CollisionGroup: string
	CollisionGroupId: number
	Color: Color3
	CurrentPhysicalProperties: PhysicalProperties
	CustomPhysicalProperties: PhysicalProperties
	Elasticity: number
	EnableFluidForces: boolean
	ExtentsCFrame: CFrame
	ExtentsSize: Vector3
	Friction: number
	FrontParamA: number
	FrontParamB: number
	FrontSurface: EnumSurfaceType
	FrontSurfaceInput: EnumInputType
	LeftParamA: number
	LeftParamB: number
	LeftSurface: EnumSurfaceType
	LeftSurfaceInput: EnumInputType
	LocalTransparencyModifier: number
	Locked: boolean
	Mass: number
	Massless: boolean
	Material: EnumMaterial
	MaterialVariant: string
	Orientation: Vector3
	PivotOffset: CFrame
	Position: Vector3
	ReceiveAge: number
	Reflectance: number
	ResizeIncrement: number
	ResizeableFaces: Faces
	RightParamA: number
	RightParamB: number
	RightSurface: EnumSurfaceType
	RightSurfaceInput: EnumInputType
	RootPriority: number
	RotVelocity: Vector3
	Rotation: Vector3
	Size: Vector3
	SpecificGravity: number
	TopParamA: number
	TopParamB: number
	TopSurface: EnumSurfaceType
	TopSurfaceInput: EnumInputType
	Transparency: number
	Velocity: Vector3
	brickColor: BrickColor
end

declare extern type BasePlayerGui extends Instance with
end

declare extern type BaseRemoteEvent extends Instance with
end

declare extern type BaseScript extends LuaSourceContainer with
	Disabled: boolean
	Enabled: boolean
	LinkedSource: string
	RunContext: EnumRunContext
end

declare extern type BaseWrap extends Instance with
	CageMeshContent: Content
	CageMeshId: string
	CageOrigin: CFrame
	CageOriginWorld: CFrame
	ImportOrigin: CFrame
	ImportOriginWorld: CFrame
end

declare extern type Beam extends Instance with
	Attachment0: Instance?
	Attachment1: Instance?
	Brightness: number
	Color: ColorSequence
	CurveSize0: number
	CurveSize1: number
	Enabled: boolean
	FaceCamera: boolean
	LightEmission: number
	LightInfluence: number
	LocalTransparencyModifier: number
	Segments: number
	Texture: string
	TextureContent: Content
	TextureLength: number
	TextureMode: EnumTextureMode
	TextureSpeed: number
	Transparency: NumberSequence
	Width0: number
	Width1: number
	ZOffset: number
end

declare extern type DataModelMesh extends Instance with
	Offset: Vector3
	Scale: Vector3
	VertexColor: Vector3
end

declare extern type BevelMesh extends DataModelMesh with
end

declare extern type BillboardGui extends LayerCollector with
	Active: boolean
	Adornee: Instance?
	AlwaysOnTop: boolean
	Brightness: number
	ClipsDescendants: boolean
	CurrentDistance: number
	DistanceLowerLimit: number
	DistanceStep: number
	DistanceUpperLimit: number
	ExtentsOffset: Vector3
	ExtentsOffsetWorldSpace: Vector3
	LightInfluence: number
	MaxDistance: number
	PlayerToHideFrom: Instance?
	Size: UDim2
	SizeOffset: Vector2
	StudsOffset: Vector3
	StudsOffsetWorldSpace: Vector3
end

declare extern type ValueBase extends Instance with
end

declare extern type BinaryStringValue extends ValueBase with
end

declare extern type BindableEvent extends Instance with
end

declare extern type BindableFunction extends Instance with
end

declare extern type BlockMesh extends BevelMesh with
end

declare extern type PostEffect extends Instance with
	Enabled: boolean
end

declare extern type BloomEffect extends PostEffect with
	Intensity: number
	Size: number
	Threshold: number
end

declare extern type BlurEffect extends PostEffect with
	Size: number
end

declare extern type BodyMover extends Instance with
end

declare extern type BodyAngularVelocity extends BodyMover with
	AngularVelocity: Vector3
	MaxTorque: Vector3
	P: number
	angularvelocity: Vector3
	maxTorque: Vector3
end

declare extern type CharacterAppearance extends Instance with
end

declare extern type BodyColors extends CharacterAppearance with
	HeadColor: BrickColor
	HeadColor3: Color3
	LeftArmColor: BrickColor
	LeftArmColor3: Color3
	LeftLegColor: BrickColor
	LeftLegColor3: Color3
	RightArmColor: BrickColor
	RightArmColor3: Color3
	RightLegColor: BrickColor
	RightLegColor3: Color3
	TorsoColor: BrickColor
	TorsoColor3: Color3
end

declare extern type BodyForce extends BodyMover with
	Force: Vector3
	force: Vector3
end

declare extern type BodyGyro extends BodyMover with
	CFrame: CFrame
	D: number
	MaxTorque: Vector3
	P: number
	cframe: CFrame
	maxTorque: Vector3
end

declare extern type BodyPartDescription extends Instance with
	AssetId: number
	BodyPart: EnumBodyPart
	Color: Color3
	HeadShape: string
	Instance: Instance?
end

declare extern type BodyPosition extends BodyMover with
	D: number
	MaxForce: Vector3
	P: number
	Position: Vector3
	maxForce: Vector3
	position: Vector3
end

declare extern type BodyThrust extends BodyMover with
	Force: Vector3
	Location: Vector3
	force: Vector3
	location: Vector3
end

declare extern type BodyVelocity extends BodyMover with
	MaxForce: Vector3
	P: number
	Velocity: Vector3
	maxForce: Vector3
	velocity: Vector3
end

declare extern type Bone extends Attachment with
	Transform: CFrame
	TransformedCFrame: CFrame
	TransformedWorldCFrame: CFrame
end

declare extern type BoolValue extends ValueBase with
	Value: boolean
end

declare extern type PVAdornment extends GuiBase3d with
	Adornee: Instance?
end

declare extern type HandleAdornment extends PVAdornment with
	AdornCullingMode: EnumAdornCullingMode
	AlwaysOnTop: boolean
	CFrame: CFrame
	SizeRelativeOffset: Vector3
	ZIndex: number
end

declare extern type BoxHandleAdornment extends HandleAdornment with
	Shading: EnumAdornShading
	Size: Vector3
end

declare extern type Breakpoint extends Instance with
end

declare extern type BrickColorValue extends ValueBase with
	Value: BrickColor
end

declare extern type BrowserService extends Instance with
end

declare extern type TextChatConfigurations extends Instance with
end

declare extern type BubbleChatConfiguration extends TextChatConfigurations with
	AdorneeName: string
	BackgroundColor3: Color3
	BackgroundTransparency: number
	BubbleDuration: number
	BubblesSpacing: number
	Enabled: boolean
	Font: EnumFont
	FontFace: Font
	LocalPlayerStudsOffset: Vector3
	MaxBubbles: number
	MaxDistance: number
	MinimizeDistance: number
	TailVisible: boolean
	TextColor3: Color3
	TextSize: number
	VerticalStudsOffset: number
end

declare extern type TextChatMessageProperties extends Instance with
	PrefixText: string
	Text: string
	Translation: string
end

declare extern type BubbleChatMessageProperties extends TextChatMessageProperties with
	BackgroundColor3: Color3
	BackgroundTransparency: number
	FontFace: Font
	TailVisible: boolean
	TextColor3: Color3
	TextSize: number
end

declare extern type BugReporterService extends Instance with
end

declare extern type BulkImportService extends Instance with
end

declare extern type BuoyancySensor extends SensorBase with
	FullySubmerged: boolean
	TouchingSurface: boolean
end

declare extern type CFrameValue extends ValueBase with
	Value: CFrame
end

declare extern type FlyweightService extends Instance with
end

declare extern type CSGDictionaryService extends FlyweightService with
end

declare extern type CacheableContentProvider extends Instance with
end

declare extern type CalloutService extends Instance with
end

declare extern type Camera extends PVInstance with
	CFrame: CFrame
	CameraSubject: Instance?
	CameraType: EnumCameraType
	CoordinateFrame: CFrame
	DiagonalFieldOfView: number
	FieldOfView: number
	FieldOfViewMode: EnumFieldOfViewMode
	Focus: CFrame
	HeadLocked: boolean
	HeadScale: number
	MaxAxisFieldOfView: number
	NearPlaneZ: number
	VRTiltAndRollEnabled: boolean
	ViewportSize: Vector2
	focus: CFrame
end

declare extern type GuiObject extends GuiBase2d with
	Active: boolean
	AnchorPoint: Vector2
	AutomaticSize: EnumAutomaticSize
	BackgroundColor: BrickColor
	BackgroundColor3: Color3
	BackgroundTransparency: number
	BlendingMode: EnumBlendMode
	BorderColor: BrickColor
	BorderColor3: Color3
	BorderMode: EnumBorderMode
	BorderSizePixel: number
	ClipsDescendants: boolean
	Draggable: boolean
	GuiState: EnumGuiState
	InputSink: EnumInputSink
	Interactable: boolean
	LayoutOrder: number
	NextSelectionDown: Instance?
	NextSelectionLeft: Instance?
	NextSelectionRight: Instance?
	NextSelectionUp: Instance?
	Position: UDim2
	Rotation: number
	Selectable: boolean
	SelectionImageObject: Instance?
	SelectionOrder: number
	Size: UDim2
	SizeConstraint: EnumSizeConstraint
	Transparency: number
	Visible: boolean
	ZIndex: number
end

declare extern type CanvasGroup extends GuiObject with
	GroupColor3: Color3
	GroupTransparency: number
end

declare extern type Capture extends Object with
	CaptureType: EnumCaptureType
	LocalId: string
	SourcePlaceId: number
	SourceUniverseId: number
end

declare extern type CaptureService extends Instance with
end

declare extern type CapturesPages extends Pages with
end

declare extern type CapturesViewConfiguration extends BaseCoreGuiConfiguration with
	Open: boolean
end

declare extern type CatalogPages extends Pages with
end

declare extern type ChangeHistoryService extends Instance with
end

declare extern type ChangeHistoryStreamingService extends Instance with
end

declare extern type ChannelSelectorSoundEffect extends CustomSoundEffect with
	Channel: number
end

declare extern type ChannelTabsConfiguration extends TextChatConfigurations with
	AbsolutePosition: Vector2
	AbsoluteSize: Vector2
	BackgroundColor3: Color3
	BackgroundTransparency: number
	Enabled: boolean
	FontFace: Font
	HoverBackgroundColor3: Color3
	SelectedTabTextColor3: Color3
	TextColor3: Color3
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
end

declare extern type CharacterMesh extends CharacterAppearance with
	BaseTextureContent: Content
	BaseTextureId: number
	BodyPart: EnumBodyPart
	MeshContent: Content
	MeshId: number
	OverlayTextureContent: Content
	OverlayTextureId: number
end

declare extern type Chat extends Instance with
	BubbleChatEnabled: boolean
	LoadDefaultChat: boolean
end

declare extern type ChatInputBarConfiguration extends TextChatConfigurations with
	AbsolutePosition: Vector2
	AbsoluteSize: Vector2
	AutocompleteEnabled: boolean
	BackgroundColor3: Color3
	BackgroundTransparency: number
	Enabled: boolean
	FontFace: Font
	IsFocused: boolean
	KeyboardKeyCode: EnumKeyCode
	PlaceholderColor3: Color3
	TargetTextChannel: Instance?
	TextBox: Instance?
	TextColor3: Color3
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
end

declare extern type ChatWindowConfiguration extends TextChatConfigurations with
	AbsolutePosition: Vector2
	AbsoluteSize: Vector2
	BackgroundColor3: Color3
	BackgroundTransparency: number
	Enabled: boolean
	FontFace: Font
	HeightScale: number
	HorizontalAlignment: EnumHorizontalAlignment
	TextColor3: Color3
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
	VerticalAlignment: EnumVerticalAlignment
	WidthScale: number
end

declare extern type ChatWindowMessageProperties extends TextChatMessageProperties with
	FontFace: Font
	PrefixTextProperties: Instance?
	TextColor3: Color3
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
end

declare extern type ChorusSoundEffect extends SoundEffect with
	Depth: number
	Mix: number
	Rate: number
end

declare extern type ClickDetector extends Instance with
	CursorIcon: string
	CursorIconContent: Content
	MaxActivationDistance: number
end

declare extern type NetworkReplicator extends Instance with
end

declare extern type ClientReplicator extends NetworkReplicator with
end

declare extern type ClientStorageService extends Instance with
end

declare extern type ClimbController extends ControllerBase with
	AccelerationTime: number
	BalanceMaxTorque: number
	BalanceSpeed: number
	MoveMaxForce: number
end

declare extern type Clothing extends CharacterAppearance with
	Color3: Color3
end

declare extern type CloudCRUDService extends Instance with
end

declare extern type CloudExecutionService extends Instance with
end

declare extern type LocalizationTable extends Instance with
	Contents: string
	DevelopmentLanguage: string
	Root: Instance?
	SourceLocaleId: string
end

declare extern type CloudLocalizationTable extends LocalizationTable with
end

declare extern type Clouds extends Instance with
	Color: Color3
	Cover: number
	Density: number
	Enabled: boolean
end

declare extern type ClusterPacketCache extends Instance with
end

declare extern type Collaborator extends Instance with
	CFrame: CFrame
	CollaboratorColor: number
	CollaboratorColor3: Color3
	CurDocGUID: string
	CurScriptLineNumber: number
	IsIdle: boolean
	Status: EnumCollaboratorStatus
	UserId: number
	Username: string
end

declare extern type CollaboratorsService extends Instance with
end

declare extern type CollectionService extends Instance with
end

declare extern type Color3Value extends ValueBase with
	Value: Color3
end

declare extern type ColorCorrectionEffect extends PostEffect with
	Brightness: number
	Contrast: number
	Saturation: number
	TintColor: Color3
end

declare extern type ColorGradingEffect extends PostEffect with
	TonemapperPreset: EnumTonemapperPreset
end

declare extern type CommerceService extends Instance with
end

declare extern type CompositeValueCurve extends Instance with
	CurveType: EnumCompositeValueCurveType
end

declare extern type CompressorSoundEffect extends SoundEffect with
	Attack: number
	GainMakeup: number
	Ratio: number
	Release: number
	SideChain: Instance?
	Threshold: number
end

declare extern type ConeHandleAdornment extends HandleAdornment with
	Height: number
	Hollow: boolean
	Radius: number
	Shading: EnumAdornShading
end

declare extern type ConfigService extends Instance with
end

declare extern type ConfigSnapshot extends Object with
	Error: EnumConfigSnapshotErrorState
	Outdated: boolean
end

declare extern type Configuration extends Instance with
end

declare extern type ConfigureServerService extends Instance with
end

declare extern type ConnectivityService extends Instance with
end

declare extern type ContentProvider extends Instance with
	BaseUrl: string
	RequestQueueSize: number
end

declare extern type ContextActionService extends Instance with
end

declare extern type Controller extends Instance with
end

declare extern type ControllerManager extends Instance with
	ActiveController: Instance?
	BaseMoveSpeed: number
	BaseTurnSpeed: number
	ClimbSensor: Instance?
	FacingDirection: Vector3
	GroundSensor: Instance?
	MovingDirection: Vector3
	RootPart: Instance?
	UpDirection: Vector3
end

declare extern type ControllerSensor extends SensorBase with
end

declare extern type ControllerPartSensor extends ControllerSensor with
	HitFrame: CFrame
	HitNormal: Vector3
	LadderSearchHeight: number
	LadderSearchOffset: number
	SearchDistance: number
	SensedMaterial: EnumMaterial
	SensedPart: Instance?
	SensorMode: EnumSensorMode
end

declare extern type ControllerService extends Instance with
end

declare extern type CookiesService extends Instance with
end

declare extern type CoreGui extends BasePlayerGui with
	Version: number
end

declare extern type CoreGuiConfiguration extends Instance with
	CapturesViewConfiguration: Instance?
	PlayerListConfiguration: Instance?
	SelfViewConfiguration: Instance?
end

declare extern type CorePackages extends Instance with
end

declare extern type CoreScript extends BaseScript with
end

declare extern type CoreScriptDebuggingManagerHelper extends Instance with
end

declare extern type CoreScriptSyncService extends Instance with
end

declare extern type CornerWedgePart extends BasePart with
end

declare extern type CreationDBService extends Instance with
end

declare extern type CreatorStoreService extends Instance with
end

declare extern type CrossDMScriptChangeListener extends Instance with
end

declare extern type CurveAnimation extends AnimationClip with
end

declare extern type CustomEvent extends Instance with
end

declare extern type CustomEventReceiver extends Instance with
	Source: Instance?
end

declare extern type CustomLog extends Instance with
end

declare extern type CylinderHandleAdornment extends HandleAdornment with
	Angle: number
	Height: number
	InnerRadius: number
	Radius: number
	Shading: EnumAdornShading
end

declare extern type CylinderMesh extends BevelMesh with
end

declare extern type SlidingBallConstraint extends Constraint with
	ActuatorType: EnumActuatorType
	CurrentPosition: number
	LimitsEnabled: boolean
	LinearResponsiveness: number
	LowerLimit: number
	MotorMaxAcceleration: number
	MotorMaxForce: number
	Restitution: number
	ServoMaxForce: number
	Size: number
	SoftlockServoUponReachingTarget: boolean
	Speed: number
	TargetPosition: number
	UpperLimit: number
	Velocity: number
end

declare extern type CylindricalConstraint extends SlidingBallConstraint with
	AngularActuatorType: EnumActuatorType
	AngularLimitsEnabled: boolean
	AngularResponsiveness: number
	AngularRestitution: number
	AngularSpeed: number
	AngularVelocity: number
	CurrentAngle: number
	InclinationAngle: number
	LowerAngle: number
	MotorMaxAngularAcceleration: number
	MotorMaxTorque: number
	RotationAxisVisible: boolean
	ServoMaxTorque: number
	SoftlockAngularServoUponReachingTarget: boolean
	TargetAngle: number
	UpperAngle: number
	WorldRotationAxis: Vector3
end

declare extern type ServiceProvider extends Instance with
end

declare extern type DataModel extends ServiceProvider with
	CreatorId: number
	CreatorType: EnumCreatorType
	GameId: number
	GearGenreSetting: EnumGearGenreSetting
	Genre: EnumGenre
	JobId: string
	MatchmakingType: EnumMatchmakingType
	PlaceId: number
	PlaceVersion: number
	PrivateServerId: string
	PrivateServerOwnerId: number
	RunService: Instance?
	VIPServerId: string
	VIPServerOwnerId: number
	Workspace: Instance?
	lighting: Instance?
	workspace: Instance?
end

declare extern type DataModelPatchService extends Instance with
end

declare extern type DataModelSession extends Instance with
end

declare extern type GlobalDataStore extends Instance with
end

declare extern type DataStore extends GlobalDataStore with
end

declare extern type DataStoreGetOptions extends Instance with
	UseCache: boolean
end

declare extern type DataStoreIncrementOptions extends Instance with
end

declare extern type DataStoreInfo extends Instance with
	CreatedTime: number
	DataStoreName: string
	UpdatedTime: number
end

declare extern type DataStoreKey extends Instance with
	KeyName: string
end

declare extern type DataStoreKeyInfo extends Instance with
	CreatedTime: number
	UpdatedTime: number
	Version: string
end

declare extern type DataStoreKeyPages extends Pages with
	Cursor: string
end

declare extern type DataStoreListingPages extends Pages with
	Cursor: string
end

declare extern type DataStoreObjectVersionInfo extends Instance with
	CreatedTime: number
	IsDeleted: boolean
	Version: string
end

declare extern type DataStoreOptions extends Instance with
	AllScopes: boolean
end

declare extern type DataStorePages extends Pages with
end

declare extern type DataStoreService extends Instance with
end

declare extern type DataStoreSetOptions extends Instance with
end

declare extern type DataStoreVersionPages extends Pages with
end

declare extern type Debris extends Instance with
	MaxItems: number
end

declare extern type DebugSettings extends Instance with
	DataModel: number
	InstanceCount: number
	IsScriptStackTracingEnabled: boolean
	JobCount: number
	PlayerCount: number
	ReportSoundWarnings: boolean
	RobloxVersion: string
	TickCountPreciseOverride: EnumTickCountSampleMethod
end

declare extern type DebuggablePluginWatcher extends Instance with
end

declare extern type DebuggerBreakpoint extends Instance with
	Condition: string
	ContinueExecution: boolean
	IsEnabled: boolean
	Line: number
	LogExpression: string
	isContextDependentBreakpoint: boolean
end

declare extern type DebuggerConnection extends Instance with
end

declare extern type DebuggerConnectionManager extends Instance with
end

declare extern type DebuggerLuaResponse extends Instance with
end

declare extern type DebuggerManager extends Instance with
	DebuggingEnabled: boolean
end

declare extern type DebuggerUIService extends Instance with
end

declare extern type DebuggerVariable extends Instance with
end

declare extern type DebuggerWatch extends Instance with
	Expression: string
end

declare extern type FaceInstance extends Instance with
	Face: EnumNormalId
end

declare extern type Decal extends FaceInstance with
	AutoLocalize: boolean
	Color3: Color3
	ColorMap: string
	ColorMapContent: Content
	LocalTransparencyModifier: number
	MetalnessMap: string
	MetalnessMapContent: Content
	NormalMap: string
	NormalMapContent: Content
	Rotation: number
	RoughnessMap: string
	RoughnessMapContent: Content
	Shiny: number
	Specular: number
	Texture: string
	TextureContent: Content
	Transparency: number
	UVOffset: Vector2
	UVScale: Vector2
	ZIndex: number
end

declare extern type DeferredAssetManagerService extends Instance with
end

declare extern type DepthOfFieldEffect extends PostEffect with
	FarIntensity: number
	FocusDistance: number
	InFocusRadius: number
	NearIntensity: number
end

declare extern type DesignFoundationsService extends Instance with
end

declare extern type DeviceIdService extends Instance with
end

declare extern type Dialog extends Instance with
	BehaviorType: EnumDialogBehaviorType
	ConversationDistance: number
	GoodbyeChoiceActive: boolean
	GoodbyeDialog: string
	InUse: boolean
	InitialPrompt: string
	Purpose: EnumDialogPurpose
	Tone: EnumDialogTone
	TriggerDistance: number
	TriggerOffset: Vector3
end

declare extern type DialogChoice extends Instance with
	GoodbyeChoiceActive: boolean
	GoodbyeDialog: string
	ResponseDialog: string
	UserDialog: string
end

declare extern type DigitsRigDescription extends Instance with
	Index1: Instance?
	Index1TposeAdjustment: CFrame
	Index2: Instance?
	Index2TposeAdjustment: CFrame
	Index3: Instance?
	Index3TposeAdjustment: CFrame
	IndexRange: Vector3
	IndexSize: number
	Middle1: Instance?
	Middle1TposeAdjustment: CFrame
	Middle2: Instance?
	Middle2TposeAdjustment: CFrame
	Middle3: Instance?
	Middle3TposeAdjustment: CFrame
	MiddleRange: Vector3
	MiddleSize: number
	Pinky1: Instance?
	Pinky1TposeAdjustment: CFrame
	Pinky2: Instance?
	Pinky2TposeAdjustment: CFrame
	Pinky3: Instance?
	Pinky3TposeAdjustment: CFrame
	PinkyRange: Vector3
	PinkySize: number
	Ring1: Instance?
	Ring1TposeAdjustment: CFrame
	Ring2: Instance?
	Ring2TposeAdjustment: CFrame
	Ring3: Instance?
	Ring3TposeAdjustment: CFrame
	RingRange: Vector3
	RingSize: number
	Side: EnumDigitsRigDescriptionSide
	Thumb1: Instance?
	Thumb1TposeAdjustment: CFrame
	Thumb2: Instance?
	Thumb2TposeAdjustment: CFrame
	Thumb3: Instance?
	Thumb3TposeAdjustment: CFrame
	ThumbRange: Vector3
	ThumbSize: number
end

declare extern type DistortionSoundEffect extends SoundEffect with
	Level: number
end

declare extern type PluginGui extends LayerCollector with
	Title: string
end

declare extern type DockWidgetPluginGui extends PluginGui with
	HostWidgetWasRestored: boolean
end

declare extern type DoubleConstrainedValue extends ValueBase with
	ConstrainedValue: number
	MaxValue: number
	MinValue: number
	Value: number
end

declare extern type DraftsService extends Instance with
end

declare extern type DragDetector extends ClickDetector with
	ActivatedCursorIcon: string
	ActivatedCursorIconContent: Content
	ApplyAtCenterOfMass: boolean
	Axis: Vector3
	DragFrame: CFrame
	DragStyle: EnumDragDetectorDragStyle
	Enabled: boolean
	GamepadModeSwitchKeyCode: EnumKeyCode
	KeyboardModeSwitchKeyCode: EnumKeyCode
	MaxDragAngle: number
	MaxDragTranslation: Vector3
	MaxForce: number
	MaxTorque: number
	MinDragAngle: number
	MinDragTranslation: Vector3
	Orientation: Vector3
	PermissionPolicy: EnumDragDetectorPermissionPolicy
	ReferenceInstance: Instance?
	ResponseStyle: EnumDragDetectorResponseStyle
	Responsiveness: number
	RunLocally: boolean
	SecondaryAxis: Vector3
	TrackballRadialPullFactor: number
	TrackballRollFactor: number
	VRSwitchKeyCode: EnumKeyCode
	WorldAxis: Vector3
	WorldSecondaryAxis: Vector3
end

declare extern type Dragger extends Instance with
end

declare extern type DraggerService extends Instance with
	AlignDraggedObjects: boolean
	AngleSnapEnabled: boolean
	AngleSnapIncrement: number
	AnimateHover: boolean
	CollisionsEnabled: boolean
	DraggerCoordinateSpace: EnumDraggerCoordinateSpace
	DraggerMovementMode: EnumDraggerMovementMode
	GeometrySnapColor: Color3
	HoverAnimateFrequency: number
	HoverThickness: number
	JointsEnabled: boolean
	LinearSnapEnabled: boolean
	LinearSnapIncrement: number
	ShowHover: boolean
	ShowPivotIndicator: boolean
end

declare extern type JointInstance extends Instance with
	Active: boolean
	C0: CFrame
	C1: CFrame
	Enabled: boolean
	Part0: Instance?
	Part1: Instance?
	part1: Instance?
end

declare extern type DynamicRotate extends JointInstance with
	BaseAngle: number
end

declare extern type EchoSoundEffect extends SoundEffect with
	Delay: number
	DryLevel: number
	Feedback: number
	WetLevel: number
end

declare extern type EditableImage extends Object with
	Size: Vector2
end

declare extern type EditableMesh extends Object with
	FixedSize: boolean
end

declare extern type EditableService extends Instance with
end

declare extern type EncodingService extends Instance with
end

declare extern type EqualizerSoundEffect extends SoundEffect with
	HighGain: number
	LowGain: number
	MidGain: number
end

declare extern type EulerRotationCurve extends Instance with
	RotationOrder: EnumRotationOrder
end

declare extern type EventIngestService extends Instance with
end

declare extern type ExampleV2Service extends Instance with
end

declare extern type ExecutedRemoteCommand extends Object with
end

declare extern type ExperienceAuthService extends Instance with
end

declare extern type ExperienceInviteOptions extends Instance with
	InviteMessageId: string
	InviteUser: number
	LaunchData: string
	PromptMessage: string
end

declare extern type ExperienceNotificationService extends Instance with
end

declare extern type ExperienceService extends Instance with
end

declare extern type ExperienceStateCaptureService extends Instance with
end

declare extern type ExperienceStateRecordingService extends Instance with
end

declare extern type ExplorerFilter extends Instance with
end

declare extern type ExplorerFilterAutocompleter extends Instance with
end

declare extern type ExplorerServiceVisibilityService extends Instance with
end

declare extern type Explosion extends Instance with
	BlastPressure: number
	BlastRadius: number
	DestroyJointRadiusPercent: number
	ExplosionType: EnumExplosionType
	LocalTransparencyModifier: number
	Position: Vector3
	TimeScale: number
	Visible: boolean
end

declare extern type FaceAnimatorService extends Instance with
end

declare extern type FaceControls extends Instance with
	ChinRaiser: number
	ChinRaiserUpperLip: number
	Corrugator: number
	EyesLookDown: number
	EyesLookLeft: number
	EyesLookRight: number
	EyesLookUp: number
	FlatPucker: number
	Funneler: number
	JawDrop: number
	JawLeft: number
	JawRight: number
	LeftBrowLowerer: number
	LeftCheekPuff: number
	LeftCheekRaiser: number
	LeftDimpler: number
	LeftEyeClosed: number
	LeftEyeUpperLidRaiser: number
	LeftInnerBrowRaiser: number
	LeftLipCornerDown: number
	LeftLipCornerPuller: number
	LeftLipStretcher: number
	LeftLowerLipDepressor: number
	LeftNoseWrinkler: number
	LeftOuterBrowRaiser: number
	LeftUpperLipRaiser: number
	LipPresser: number
	LipsTogether: number
	LowerLipSuck: number
	MouthLeft: number
	MouthRight: number
	Pucker: number
	RightBrowLowerer: number
	RightCheekPuff: number
	RightCheekRaiser: number
	RightDimpler: number
	RightEyeClosed: number
	RightEyeUpperLidRaiser: number
	RightInnerBrowRaiser: number
	RightLipCornerDown: number
	RightLipCornerPuller: number
	RightLipStretcher: number
	RightLowerLipDepressor: number
	RightNoseWrinkler: number
	RightOuterBrowRaiser: number
	RightUpperLipRaiser: number
	TongueDown: number
	TongueOut: number
	TongueUp: number
	UpperLipSuck: number
end

declare extern type FacialAgeEstimationService extends Instance with
end

declare extern type FacialAnimationRecordingService extends Instance with
end

declare extern type FacialAnimationStreamingServiceStats extends Instance with
end

declare extern type FacialAnimationStreamingServiceV2 extends Instance with
end

declare extern type FacialAnimationStreamingSubsessionStats extends Instance with
end

declare extern type FacsImportData extends BaseImportData with
end

declare extern type Feature extends Instance with
	FaceId: EnumNormalId
	InOut: EnumInOut
	LeftRight: EnumLeftRight
	TopBottom: EnumTopBottom
end

declare extern type FeatureRestrictionManager extends Instance with
end

declare extern type File extends Instance with
	Size: number
end

declare extern type FileManagerService extends Instance with
end

declare extern type FileMesh extends DataModelMesh with
	MeshContent: Content
	MeshId: string
	TextureContent: Content
	TextureId: string
end

declare extern type FileSyncReplicationService extends Instance with
end

declare extern type Fire extends Instance with
	Color: Color3
	Enabled: boolean
	Heat: number
	LocalTransparencyModifier: number
	SecondaryColor: Color3
	Size: number
	TimeScale: number
	size: number
end

declare extern type Tool extends BackpackItem with
	CanBeDropped: boolean
	Enabled: boolean
	Grip: CFrame
	GripForward: Vector3
	GripPos: Vector3
	GripRight: Vector3
	GripUp: Vector3
	ManualActivationOnly: boolean
	RequiresHandle: boolean
	ToolTip: string
end

declare extern type Flag extends Tool with
	TeamColor: BrickColor
end

declare extern type FormFactorPart extends BasePart with
	FormFactor: EnumFormFactor
	formFactor: EnumFormFactor
end

declare extern type Part extends FormFactorPart with
	Shape: EnumPartType
end

declare extern type FlagStand extends Part with
	TeamColor: BrickColor
end

declare extern type FlagStandService extends Instance with
end

declare extern type FlangeSoundEffect extends SoundEffect with
	Depth: number
	Mix: number
	Rate: number
end

declare extern type FloatCurve extends Instance with
	Length: number
end

declare extern type FloorWire extends GuiBase3d with
	CycleOffset: number
	From: Instance?
	StudsBetweenTextures: number
	Texture: string
	TextureSize: Vector2
	To: Instance?
	Velocity: number
	WireRadius: number
end

declare extern type FluidForceSensor extends SensorBase with
	CenterOfPressure: Vector3
	Force: Vector3
	Torque: Vector3
end

declare extern type Folder extends Instance with
end

declare extern type ForceField extends Instance with
	Visible: boolean
end

declare extern type Frame extends GuiObject with
	Style: EnumFrameStyle
end

declare extern type FriendPages extends Pages with
end

declare extern type FriendService extends Instance with
end

declare extern type FunctionalTest extends Instance with
	Description: string
end

declare extern type GamePassService extends Instance with
end

declare extern type GameSettings extends Instance with
	VideoCaptureEnabled: boolean
end

declare extern type GamepadService extends Instance with
	GamepadCursorEnabled: boolean
end

declare extern type GeneratedFolder extends Folder with
end

declare extern type GenerationService extends Instance with
end

declare extern type GenericChallengeService extends Instance with
end

declare extern type GenericSettings extends ServiceProvider with
end

declare extern type Geometry extends Instance with
end

declare extern type GeometryService extends Instance with
end

declare extern type GetTextBoundsParams extends Instance with
	Font: Font
	RichText: boolean
	Size: number
	Text: string
	Width: number
end

declare extern type GlobalSettings extends GenericSettings with
end

declare extern type Glue extends JointInstance with
	F0: Vector3
	F1: Vector3
	F2: Vector3
	F3: Vector3
end

declare extern type GongService extends Instance with
end

declare extern type GroundController extends ControllerBase with
	AccelerationLean: number
	AccelerationTime: number
	BalanceMaxTorque: number
	BalanceSpeed: number
	DecelerationTime: number
	Friction: number
	FrictionWeight: number
	GroundOffset: number
	StandForce: number
	StandSpeed: number
	TurnSpeedFactor: number
end

declare extern type GroupImportData extends BaseImportData with
	Anchored: boolean
	ImportAsModelAsset: boolean
	InsertInWorkspace: boolean
end

declare extern type GroupService extends Instance with
end

declare extern type GuiButton extends GuiObject with
	AutoButtonColor: boolean
	HoverHapticEffect: Instance?
	Modal: boolean
	PressHapticEffect: Instance?
	Selected: boolean
	Style: EnumButtonStyle
end

declare extern type GuiLabel extends GuiObject with
end

declare extern type ScreenGui extends LayerCollector with
	ClipToDeviceSafeArea: boolean
	DisplayOrder: number
	IgnoreGuiInset: boolean
	SafeAreaCompatibility: EnumSafeAreaCompatibility
	ScreenInsets: EnumScreenInsets
end

declare extern type GuiMain extends ScreenGui with
end

declare extern type GuiService extends Instance with
	AutoSelectGuiEnabled: boolean
	CoreGuiNavigationEnabled: boolean
	GuiNavigationEnabled: boolean
	IsModalDialog: boolean
	IsWindows: boolean
	MenuIsOpen: boolean
	PreferredTextSize: EnumPreferredTextSize
	PreferredTransparency: number
	ReducedMotionEnabled: boolean
	SelectedObject: Instance?
	TopbarInset: Rect
	TouchControlsEnabled: boolean
	ViewportDisplaySize: EnumDisplaySize
end

declare extern type GuidRegistryService extends Instance with
end

declare extern type HSRDataContentProvider extends CacheableContentProvider with
end

declare extern type Handles extends HandlesBase with
	Faces: Faces
	Style: EnumHandlesStyle
end

declare extern type HapticEffect extends Instance with
	Looped: boolean
	Position: Vector3
	Radius: number
	Type: EnumHapticEffectType
end

declare extern type HapticService extends Instance with
end

declare extern type HarmonyService extends Instance with
end

declare extern type Hat extends Accoutrement with
end

declare extern type HeapProfilerService extends Instance with
end

declare extern type HeatmapQueryService extends Instance with
end

declare extern type HeatmapService extends Instance with
end

declare extern type HeightmapImporterService extends Instance with
end

declare extern type HiddenSurfaceRemovalAsset extends Instance with
end

declare extern type Highlight extends Instance with
	Adornee: Instance?
	DepthMode: EnumHighlightDepthMode
	Enabled: boolean
	FillColor: Color3
	FillTransparency: number
	OutlineColor: Color3
	OutlineTransparency: number
end

declare extern type HingeConstraint extends Constraint with
	ActuatorType: EnumActuatorType
	AngularResponsiveness: number
	AngularSpeed: number
	AngularVelocity: number
	CurrentAngle: number
	LimitsEnabled: boolean
	LowerAngle: number
	MotorMaxAcceleration: number
	MotorMaxTorque: number
	Radius: number
	Restitution: number
	ServoMaxTorque: number
	SoftlockServoUponReachingTarget: boolean
	TargetAngle: number
	UpperAngle: number
end

declare extern type Message extends Instance with
	Text: string
end

declare extern type Hint extends Message with
end

declare extern type Hole extends Feature with
end

declare extern type Hopper extends Instance with
end

declare extern type HopperBin extends BackpackItem with
	Active: boolean
	BinType: EnumBinType
end

declare extern type HttpRbxApiService extends Instance with
end

declare extern type HttpRequest extends Instance with
end

declare extern type HttpService extends Instance with
	HttpEnabled: boolean
end

declare extern type Humanoid extends Instance with
	AutoJumpEnabled: boolean
	AutoRotate: boolean
	AutomaticScalingEnabled: boolean
	BreakJointsOnDeath: boolean
	CameraOffset: Vector3
	CollisionType: EnumHumanoidCollisionType
	DisplayDistanceType: EnumHumanoidDisplayDistanceType
	DisplayName: string
	EvaluateStateMachine: boolean
	FloorMaterial: EnumMaterial
	Health: number
	HealthDisplayDistance: number
	HealthDisplayType: EnumHumanoidHealthDisplayType
	HipHeight: number
	Jump: boolean
	JumpHeight: number
	JumpPower: number
	LeftLeg: Instance?
	MaxHealth: number
	MaxSlopeAngle: number
	MoveDirection: Vector3
	NameDisplayDistance: number
	NameOcclusion: EnumNameOcclusion
	PlatformStand: boolean
	RequiresNeck: boolean
	RigType: EnumHumanoidRigType
	RightLeg: Instance?
	RootPart: Instance?
	SeatPart: Instance?
	Sit: boolean
	TargetPoint: Vector3
	Torso: Instance?
	UseJumpPower: boolean
	WalkSpeed: number
	WalkToPart: Instance?
	WalkToPoint: Vector3
	maxHealth: number
end

declare extern type HumanoidController extends Controller with
end

declare extern type HumanoidDescription extends Instance with
	BackAccessory: string
	BodyTypeScale: number
	ClimbAnimation: number
	DepthScale: number
	Face: number
	FaceAccessory: string
	FallAnimation: number
	FrontAccessory: string
	GraphicTShirt: number
	HairAccessory: string
	HatAccessory: string
	Head: number
	HeadColor: Color3
	HeadScale: number
	HeightScale: number
	IdleAnimation: number
	JumpAnimation: number
	LeftArm: number
	LeftArmColor: Color3
	LeftLeg: number
	LeftLegColor: Color3
	MoodAnimation: number
	NeckAccessory: string
	Pants: number
	ProportionScale: number
	RightArm: number
	RightArmColor: Color3
	RightLeg: number
	RightLegColor: Color3
	RunAnimation: number
	Shirt: number
	ShouldersAccessory: string
	StaticFacialAnimation: boolean
	SwimAnimation: number
	Torso: number
	TorsoColor: Color3
	UseAvatarSettings: boolean
	WaistAccessory: string
	WalkAnimation: number
	WidthScale: number
end

declare extern type HumanoidRigDescription extends Instance with
	Chest: Instance?
	ChestRangeMax: Vector3
	ChestRangeMin: Vector3
	ChestSize: number
	ChestTposeAdjustment: CFrame
	HeadBase: Instance?
	HeadBaseRangeMax: Vector3
	HeadBaseRangeMin: Vector3
	HeadBaseSize: number
	HeadBaseTposeAdjustment: CFrame
	LeftAnkle: Instance?
	LeftAnkleRangeMax: Vector3
	LeftAnkleRangeMin: Vector3
	LeftAnkleSize: number
	LeftAnkleTposeAdjustment: CFrame
	LeftClavicle: Instance?
	LeftClavicleRangeMax: Vector3
	LeftClavicleRangeMin: Vector3
	LeftClavicleSize: number
	LeftClavicleTposeAdjustment: CFrame
	LeftElbow: Instance?
	LeftElbowRangeMax: Vector3
	LeftElbowRangeMin: Vector3
	LeftElbowSize: number
	LeftElbowTposeAdjustment: CFrame
	LeftHip: Instance?
	LeftHipRangeMax: Vector3
	LeftHipRangeMin: Vector3
	LeftHipSize: number
	LeftHipTposeAdjustment: CFrame
	LeftKnee: Instance?
	LeftKneeRangeMax: Vector3
	LeftKneeRangeMin: Vector3
	LeftKneeSize: number
	LeftKneeTposeAdjustment: CFrame
	LeftShoulder: Instance?
	LeftShoulderRangeMax: Vector3
	LeftShoulderRangeMin: Vector3
	LeftShoulderSize: number
	LeftShoulderTposeAdjustment: CFrame
	LeftToeBase: Instance?
	LeftToeBaseRangeMax: Vector3
	LeftToeBaseRangeMin: Vector3
	LeftToeBaseSize: number
	LeftToeBaseTposeAdjustment: CFrame
	LeftWrist: Instance?
	LeftWristRangeMax: Vector3
	LeftWristRangeMin: Vector3
	LeftWristSize: number
	LeftWristTposeAdjustment: CFrame
	Neck: Instance?
	NeckRangeMax: Vector3
	NeckRangeMin: Vector3
	NeckSize: number
	NeckTposeAdjustment: CFrame
	RightAnkle: Instance?
	RightAnkleRangeMax: Vector3
	RightAnkleRangeMin: Vector3
	RightAnkleSize: number
	RightAnkleTposeAdjustment: CFrame
	RightClavicle: Instance?
	RightClavicleRangeMax: Vector3
	RightClavicleRangeMin: Vector3
	RightClavicleSize: number
	RightClavicleTposeAdjustment: CFrame
	RightElbow: Instance?
	RightElbowRangeMax: Vector3
	RightElbowRangeMin: Vector3
	RightElbowSize: number
	RightElbowTposeAdjustment: CFrame
	RightHip: Instance?
	RightHipRangeMax: Vector3
	RightHipRangeMin: Vector3
	RightHipSize: number
	RightHipTposeAdjustment: CFrame
	RightKnee: Instance?
	RightKneeRangeMax: Vector3
	RightKneeRangeMin: Vector3
	RightKneeSize: number
	RightKneeTposeAdjustment: CFrame
	RightShoulder: Instance?
	RightShoulderRangeMax: Vector3
	RightShoulderRangeMin: Vector3
	RightShoulderSize: number
	RightShoulderTposeAdjustment: CFrame
	RightToeBase: Instance?
	RightToeBaseRangeMax: Vector3
	RightToeBaseRangeMin: Vector3
	RightToeBaseSize: number
	RightToeBaseTposeAdjustment: CFrame
	RightWrist: Instance?
	RightWristRangeMax: Vector3
	RightWristRangeMin: Vector3
	RightWristSize: number
	RightWristTposeAdjustment: CFrame
	Root: Instance?
	RootRangeMax: Vector3
	RootRangeMin: Vector3
	RootSize: number
	RootTposeAdjustment: CFrame
	Spine: Instance?
	SpineRangeMax: Vector3
	SpineRangeMin: Vector3
	SpineSize: number
	SpineTposeAdjustment: CFrame
	Waist: Instance?
	WaistRangeMax: Vector3
	WaistRangeMin: Vector3
	WaistSize: number
	WaistTposeAdjustment: CFrame
end

declare extern type IKControl extends Instance with
	ChainRoot: Instance?
	Enabled: boolean
	EndEffector: Instance?
	EndEffectorOffset: CFrame
	Offset: CFrame
	Pole: Instance?
	Priority: number
	SmoothTime: number
	Target: Instance?
	Type: EnumIKControlType
	Weight: number
end

declare extern type ILegacyStudioBridge extends Instance with
end

declare extern type IXPService extends Instance with
end

declare extern type ImageButton extends GuiButton with
	HoverImage: string
	HoverImageContent: Content
	Image: string
	ImageColor3: Color3
	ImageContent: Content
	ImageRectOffset: Vector2
	ImageRectSize: Vector2
	ImageTransparency: number
	IsLoaded: boolean
	PressedImage: string
	PressedImageContent: Content
	ResampleMode: EnumResamplerMode
	ScaleType: EnumScaleType
	SliceCenter: Rect
	SliceScale: number
	TileSize: UDim2
end

declare extern type ImageHandleAdornment extends HandleAdornment with
	Image: string
	ImageContent: Content
	Size: Vector2
end

declare extern type ImageLabel extends GuiLabel with
	Image: string
	ImageColor3: Color3
	ImageContent: Content
	ImageRectOffset: Vector2
	ImageRectSize: Vector2
	ImageTransparency: number
	IsLoaded: boolean
	ResampleMode: EnumResamplerMode
	ScaleType: EnumScaleType
	SliceCenter: Rect
	SliceScale: number
	TileSize: UDim2
end

declare extern type ImageScreenCaptureService extends Instance with
end

declare extern type IncrementalPatchBuilder extends Instance with
	AddPathsToBundle: boolean
	BuildDebouncePeriod: number
	HighCompression: boolean
	SerializePatch: boolean
	UseFileLevelCompressionInsteadOfChunk: boolean
	ZstdCompression: boolean
end

declare extern type InputAction extends Instance with
	Enabled: boolean
	Type: EnumInputActionType
end

declare extern type InputActionLabel extends GuiObject with
	ContentText: string
	Font: EnumFont
	FontFace: Font
	FontSize: EnumFontSize
	Image: string
	ImageColor3: Color3
	ImageContent: Content
	ImageRectOffset: Vector2
	ImageRectSize: Vector2
	ImageTransparency: number
	InputAction: string
	IsLoaded: boolean
	LineHeight: number
	LocalizedText: string
	MaxVisibleGraphemes: number
	OpenTypeFeatures: string
	OpenTypeFeaturesError: string
	ResampleMode: EnumResamplerMode
	RichText: boolean
	ScaleType: EnumScaleType
	SliceCenter: Rect
	SliceScale: number
	Text: string
	TextBounds: Vector2
	TextColor: BrickColor
	TextColor3: Color3
	TextDirection: EnumTextDirection
	TextFits: boolean
	TextScaled: boolean
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
	TextTransparency: number
	TextTruncate: EnumTextTruncate
	TextWrap: boolean
	TextWrapped: boolean
	TextXAlignment: EnumTextXAlignment
	TextYAlignment: EnumTextYAlignment
	TileSize: UDim2
end

declare extern type InputBinding extends Instance with
	Backward: EnumKeyCode
	ClampMagnitudeToOne: boolean
	Down: EnumKeyCode
	Forward: EnumKeyCode
	KeyCode: EnumKeyCode
	Left: EnumKeyCode
	PointerIndex: number
	PressedThreshold: number
	PrimaryModifier: EnumKeyCode
	ReleasedThreshold: number
	ResponseCurve: number
	Right: EnumKeyCode
	Scale: number
	SecondaryModifier: EnumKeyCode
	Type: EnumInputBindingType
	UIButton: Instance?
	UIModifier: Instance?
	Up: EnumKeyCode
	Vector2Scale: Vector2
	Vector3Scale: Vector3
end

declare extern type InputContext extends Instance with
	Enabled: boolean
	Priority: number
	Sink: boolean
end

declare extern type InputObject extends Instance with
	Delta: Vector3
	KeyCode: EnumKeyCode
	Position: Vector3
	UserInputState: EnumUserInputState
	UserInputType: EnumUserInputType
end

declare extern type InsertService extends Instance with
	AllowInsertFreeModels: boolean
end

declare extern type InstanceAdornment extends GuiBase3d with
	Adornee: Instance?
end

declare extern type InstanceExtensionsService extends Instance with
end

declare extern type InstanceFileSyncService extends Instance with
end

declare extern type IntConstrainedValue extends ValueBase with
	ConstrainedValue: number
	MaxValue: number
	MinValue: number
	Value: number
end

declare extern type IntValue extends ValueBase with
	Value: number
end

declare extern type InternalMessagingService extends Instance with
end

declare extern type InternalMessagingServiceVerifier extends Instance with
end

declare extern type InternalSyncItem extends Instance with
end

declare extern type InternalSyncService extends Instance with
end

declare extern type TriangleMeshPart extends BasePart with
	CollisionFidelity: EnumCollisionFidelity
	FluidFidelity: EnumFluidFidelity
	MeshSize: Vector3
end

declare extern type PartOperation extends TriangleMeshPart with
	RenderFidelity: EnumRenderFidelity
	SmoothingAngle: number
	TriangleCount: number
	UsePartColor: boolean
end

declare extern type IntersectOperation extends PartOperation with
end

declare extern type InventoryPages extends Pages with
end

declare extern type JointImportData extends BaseImportData with
end

declare extern type JointsService extends Instance with
end

declare extern type KeyboardService extends Instance with
end

declare extern type Keyframe extends Instance with
	Time: number
end

declare extern type KeyframeMarker extends Instance with
	Value: string
end

declare extern type KeyframeSequence extends AnimationClip with
	AuthoredHipHeight: number
end

declare extern type KeyframeSequenceProvider extends Instance with
end

declare extern type LanguageService extends Instance with
end

declare extern type LegacyStudioBridge extends ILegacyStudioBridge with
end

declare extern type Light extends Instance with
	Brightness: number
	Color: Color3
	Enabled: boolean
	Shadows: boolean
end

declare extern type Lighting extends Instance with
	Ambient: Color3
	Brightness: number
	ClockTime: number
	ColorShift_Bottom: Color3
	ColorShift_Top: Color3
	EnvironmentDiffuseScale: number
	EnvironmentSpecularScale: number
	ExposureCompensation: number
	FogColor: Color3
	FogEnd: number
	FogStart: number
	GeographicLatitude: number
	GlobalShadows: boolean
	LightingStyle: EnumLightingStyle
	OutdoorAmbient: Color3
	Outlines: boolean
	PrioritizeLightingQuality: boolean
	ShadowColor: Color3
	ShadowSoftness: number
	TimeOfDay: string
end

declare extern type LineForce extends Constraint with
	ApplyAtCenterOfMass: boolean
	InverseSquareLaw: boolean
	Magnitude: number
	MaxForce: number
	ReactionForceEnabled: boolean
end

declare extern type LineHandleAdornment extends HandleAdornment with
	Length: number
	Thickness: number
end

declare extern type LinearVelocity extends Constraint with
	ForceLimitMode: EnumForceLimitMode
	ForceLimitsEnabled: boolean
	LineDirection: Vector3
	LineVelocity: number
	MaxAxesForce: Vector3
	MaxForce: number
	MaxPlanarAxesForce: Vector2
	PlaneVelocity: Vector2
	PrimaryTangentAxis: Vector3
	ReactionForceEnabled: boolean
	RelativeTo: EnumActuatorRelativeTo
	SecondaryTangentAxis: Vector3
	VectorVelocity: Vector3
	VelocityConstraintMode: EnumVelocityConstraintMode
end

declare extern type LinkingService extends Instance with
end

declare extern type LiveScriptingService extends Instance with
end

declare extern type LiveSyncService extends Instance with
	HasSyncedInstances: boolean
end

declare extern type LocalDebuggerConnection extends DebuggerConnection with
end

declare extern type Script extends BaseScript with
	Source: string
end

declare extern type LocalScript extends Script with
end

declare extern type LocalizationService extends Instance with
	RobloxLocaleId: string
	SystemLocaleId: string
end

declare extern type LodDataEntity extends Instance with
	EntityLodEnabled: boolean
end

declare extern type LodDataService extends Instance with
end

declare extern type LogReporterService extends Instance with
end

declare extern type LogService extends Instance with
end

declare extern type LoginService extends Instance with
end

declare extern type LuaSettings extends Instance with
end

declare extern type LuaWebService extends Instance with
end

declare extern type LuauScriptAnalyzerService extends Instance with
end

declare extern type MLModelDeliveryService extends Instance with
end

declare extern type MLService extends Instance with
end

declare extern type MLSession extends Object with
end

declare extern type MakeupDescription extends Instance with
	AssetId: number
	Instance: Instance?
	MakeupType: EnumMakeupType
	Order: number
end

declare extern type ManualSurfaceJointInstance extends JointInstance with
end

declare extern type ManualGlue extends ManualSurfaceJointInstance with
end

declare extern type ManualWeld extends ManualSurfaceJointInstance with
end

declare extern type MarkerCurve extends Instance with
	Length: number
end

declare extern type MarketplaceService extends Instance with
end

declare extern type MatchmakingService extends Instance with
end

declare extern type MaterialGenerationService extends Instance with
end

declare extern type MaterialImportData extends BaseImportData with
	DiffuseFilePath: string
	EmissiveFilePath: string
	IsPbr: boolean
	MetalnessFilePath: string
	NormalFilePath: string
	RoughnessFilePath: string
end

declare extern type MaterialService extends Instance with
end

declare extern type MaterialVariant extends Instance with
	AlphaMode: EnumAlphaMode
	BaseMaterial: EnumMaterial
	ColorMap: string
	ColorMapContent: Content
	CustomPhysicalProperties: PhysicalProperties
	EmissiveMaskContent: Content
	EmissiveStrength: number
	EmissiveTint: Color3
	MaterialPattern: EnumMaterialPattern
	MetalnessMap: string
	MetalnessMapContent: Content
	NormalMap: string
	NormalMapContent: Content
	RoughnessMap: string
	RoughnessMapContent: Content
	StudsPerTile: number
end

declare extern type MemStorageConnection extends Instance with
end

declare extern type MemStorageService extends Instance with
end

declare extern type MemoryStoreHashMap extends Instance with
end

declare extern type MemoryStoreHashMapPages extends Pages with
end

declare extern type MemoryStoreQueue extends Instance with
end

declare extern type MemoryStoreService extends Instance with
end

declare extern type MemoryStoreSortedMap extends Instance with
end

declare extern type MeshContentProvider extends CacheableContentProvider with
end

declare extern type MeshImportData extends BaseImportData with
	Anchored: boolean
	CageManifold: boolean
	CageMeshIntersectedPreview: boolean
	CageMeshNotIntersected: boolean
	CageNoOverlappingVertices: boolean
	CageNonManifoldPreview: boolean
	CageOverlappingVerticesPreview: boolean
	CageUVMatched: boolean
	CageUVMisMatchedPreview: boolean
	Dimensions: Vector3
	DoubleSided: boolean
	IgnoreVertexColors: boolean
	IrrelevantCageModifiedPreview: boolean
	MeshHoleDetectedPreview: boolean
	MeshNoHoleDetected: boolean
	NoIrrelevantCageModified: boolean
	NoOuterCageFarExtendedFromMesh: boolean
	OuterCageFarExtendedFromMeshPreview: boolean
	PolygonCount: number
	UseImportedPivot: boolean
end

declare extern type MeshPart extends TriangleMeshPart with
	DoubleSided: boolean
	HasJointOffset: boolean
	HasSkinnedMesh: boolean
	JointOffset: Vector3
	MeshContent: Content
	MeshId: string
	RenderFidelity: EnumRenderFidelity
	TextureContent: Content
	TextureID: string
end

declare extern type MessageBusConnection extends Instance with
end

declare extern type MessageBusService extends Instance with
end

declare extern type MessagingService extends Instance with
end

declare extern type MetaBreakpoint extends Instance with
end

declare extern type MetaBreakpointContext extends Instance with
end

declare extern type MetaBreakpointManager extends Instance with
end

declare extern type MicroProfilerService extends Instance with
end

declare extern type ModerationService extends Instance with
end

declare extern type ModuleScript extends LuaSourceContainer with
	LinkedSource: string
	Source: string
end

declare extern type Motor extends JointInstance with
	CurrentAngle: number
	DesiredAngle: number
	MaxVelocity: number
end

declare extern type Motor6D extends Motor with
	Transform: CFrame
end

declare extern type MotorFeature extends Feature with
end

declare extern type Mouse extends Instance with
	Hit: CFrame
	Icon: string
	IconContent: Content
	Origin: CFrame
	Target: Instance?
	TargetFilter: Instance?
	TargetSurface: EnumNormalId
	UnitRay: Ray
	ViewSizeX: number
	ViewSizeY: number
	X: number
	Y: number
	hit: CFrame
	target: Instance?
end

declare extern type MouseService extends Instance with
end

declare extern type MultipleDocumentInterfaceInstance extends Instance with
end

declare extern type NegateOperation extends PartOperation with
end

declare extern type NetworkPeer extends Instance with
end

declare extern type NetworkClient extends NetworkPeer with
end

declare extern type NetworkMarker extends Instance with
end

declare extern type NetworkServer extends NetworkPeer with
end

declare extern type NetworkSettings extends Instance with
	EmulatedTotalMemoryInMB: number
	FreeMemoryMBytes: number
	HttpProxyEnabled: boolean
	HttpProxyURL: string
	InboundNetworkJitterMs: number
	InboundNetworkLossPercent: number
	InboundNetworkMinDelayMs: number
	IncomingReplicationLag: number
	OutboundNetworkJitterMs: number
	OutboundNetworkLossPercent: number
	OutboundNetworkMinDelayMs: number
	PrintJoinSizeBreakdown: boolean
	PrintPhysicsErrors: boolean
	PrintStreamInstanceQuota: boolean
	RandomizeJoinInstanceOrder: boolean
	RenderStreamedRegions: boolean
	ShowActiveAnimationAsset: boolean
end

declare extern type NoCollisionConstraint extends Instance with
	Enabled: boolean
	Part0: Instance?
	Part1: Instance?
end

declare extern type Noise extends Instance with
end

declare extern type NonReplicatedCSGDictionaryService extends FlyweightService with
end

declare extern type NotificationService extends Instance with
end

declare extern type PoseBase extends Instance with
	EasingDirection: EnumPoseEasingDirection
	EasingStyle: EnumPoseEasingStyle
	Weight: number
end

declare extern type NumberPose extends PoseBase with
	Value: number
end

declare extern type NumberValue extends ValueBase with
	Value: number
end

declare extern type ObjectValue extends ValueBase with
	Value: Instance?
end

declare extern type OmniRecommendationsService extends Instance with
end

declare extern type OpenCloudApiV1 extends Instance with
end

declare extern type OpenCloudService extends Instance with
end

declare extern type OperationGraph extends Instance with
end

declare extern type OrderedDataStore extends GlobalDataStore with
end

declare extern type OutfitPages extends Pages with
end

declare extern type OutputLink extends Object with
end

declare extern type PackageLink extends Instance with
	DefaultName: string
	PackageContent: Content
	PackageId: string
	VersionNumber: number
end

declare extern type PackageService extends Instance with
end

declare extern type PackageUIService extends Instance with
end

declare extern type Packages extends Instance with
end

declare extern type Pants extends Clothing with
	PantsTemplate: string
	PantsTemplateContent: Content
end

declare extern type ParabolaAdornment extends PVAdornment with
end

declare extern type PartOperationAsset extends Instance with
end

declare extern type ParticleEmitter extends Instance with
	Acceleration: Vector3
	Brightness: number
	Color: ColorSequence
	Drag: number
	EmissionDirection: EnumNormalId
	Enabled: boolean
	FlipbookBlendFrames: boolean
	FlipbookFramerate: NumberRange
	FlipbookIncompatible: string
	FlipbookLayout: EnumParticleFlipbookLayout
	FlipbookMode: EnumParticleFlipbookMode
	FlipbookSizeX: number
	FlipbookSizeY: number
	FlipbookStartRandom: boolean
	Lifetime: NumberRange
	LightEmission: number
	LightInfluence: number
	LocalTransparencyModifier: number
	LockedToPart: boolean
	Orientation: EnumParticleOrientation
	Rate: number
	RotSpeed: NumberRange
	Rotation: NumberRange
	Shape: EnumParticleEmitterShape
	ShapeInOut: EnumParticleEmitterShapeInOut
	ShapePartial: number
	ShapeStyle: EnumParticleEmitterShapeStyle
	Size: NumberSequence
	Speed: NumberRange
	SpreadAngle: Vector2
	Squash: NumberSequence
	Texture: string
	TextureContent: Content
	TimeScale: number
	Transparency: NumberSequence
	VelocityInheritance: number
	VelocitySpread: number
	WindAffectsDrag: boolean
	ZOffset: number
end

declare extern type PartyEmulatorService extends Instance with
end

declare extern type PatchBundlerFileWatch extends Instance with
end

declare extern type PatchMapping extends Instance with
	FlattenTree: boolean
	PatchId: string
	TargetPath: string
end

declare extern type Path extends Instance with
	Status: EnumPathStatus
end

declare extern type Path2D extends GuiBase with
	Closed: boolean
	Color3: Color3
	Thickness: number
	Visible: boolean
	ZIndex: number
end

declare extern type PathfindingLink extends Instance with
	Attachment0: Instance?
	Attachment1: Instance?
	IsBidirectional: boolean
	Label: string
end

declare extern type PathfindingModifier extends Instance with
	Label: string
	PassThrough: boolean
end

declare extern type PathfindingService extends Instance with
	EmptyCutoff: number
end

declare extern type PausedState extends Instance with
end

declare extern type PausedStateBreakpoint extends PausedState with
end

declare extern type PausedStateException extends PausedState with
end

declare extern type PerformanceControlService extends Instance with
end

declare extern type PermissionsService extends Instance with
end

declare extern type PhysicsService extends Instance with
end

declare extern type PhysicsSettings extends Instance with
	AllowSleep: boolean
	AreAnchorsShown: boolean
	AreAssembliesShown: boolean
	AreAwakePartsHighlighted: boolean
	AreBodyTypesShown: boolean
	AreContactIslandsShown: boolean
	AreContactPointsShown: boolean
	AreJointCoordinatesShown: boolean
	AreMechanismsShown: boolean
	AreModelCoordsShown: boolean
	AreNonAnchorsShown: boolean
	AreOwnersShown: boolean
	ArePartCoordsShown: boolean
	AreRegionsShown: boolean
	AreTerrainReplicationRegionsShown: boolean
	AreUnalignedPartsShown: boolean
	AreWorldCoordsShown: boolean
	DisableCSGv2: boolean
	DisableCSGv3ForPlugins: boolean
	ForceCSGv2: boolean
	IsInterpolationThrottleShown: boolean
	IsReceiveAgeShown: boolean
	IsTreeShown: boolean
	PhysicsEnvironmentalThrottle: EnumEnviromentalPhysicsThrottle
	ShowDecompositionGeometry: boolean
	ThrottleAdjustTime: number
	UseCSGv2: boolean
end

declare extern type PitchShiftSoundEffect extends SoundEffect with
	Octave: number
end

declare extern type PlaceAssetIdsService extends Instance with
end

declare extern type PlaceStatsService extends Instance with
end

declare extern type PlacesService extends Instance with
end

declare extern type PlaneConstraint extends Constraint with
end

declare extern type Plane extends PlaneConstraint with
end

declare extern type Platform extends Part with
end

declare extern type PlatformCloudStorageService extends Instance with
end

declare extern type PlatformFriendsService extends Instance with
end

declare extern type PlatformLibraries extends Instance with
end

declare extern type Player extends Instance with
	AccountAge: number
	AgeChecked: EnumAgeCheckStatus
	AutoJumpEnabled: boolean
	CameraMaxZoomDistance: number
	CameraMinZoomDistance: number
	CameraMode: EnumCameraMode
	CanLoadCharacterAppearance: boolean
	Character: Instance?
	CharacterAppearance: string
	CharacterAppearanceId: number
	DataComplexity: number
	DataReady: boolean
	DevCameraOcclusionMode: EnumDevCameraOcclusionMode
	DevComputerCameraMode: EnumDevComputerCameraMovementMode
	DevComputerMovementMode: EnumDevComputerMovementMode
	DevEnableMouseLock: boolean
	DevTouchCameraMode: EnumDevTouchCameraMovementMode
	DevTouchMovementMode: EnumDevTouchMovementMode
	DisplayName: string
	FollowUserId: number
	GameplayPaused: boolean
	HasRobloxSubscription: boolean
	HasVerifiedBadge: boolean
	HealthDisplayDistance: number
	LocaleId: string
	MembershipType: EnumMembershipType
	NameDisplayDistance: number
	Neutral: boolean
	PartyId: string
	ReplicationFocus: Instance?
	RespawnLocation: Instance?
	Team: Instance?
	TeamColor: BrickColor
	UserId: number
	userId: number
end

declare extern type PlayerData extends Instance with
end

declare extern type PlayerDataRecord extends Instance with
	CreatedTime: number
	DefaultRecordName: boolean
	Dirty: boolean
	Error: EnumPlayerDataErrorState
	FlushedTime: number
	LoadedTime: number
	ModifiedTime: number
	NewRecord: boolean
	Readable: boolean
	RecordName: string
	Writable: boolean
end

declare extern type PlayerDataRecordConfig extends Instance with
	RecordName: string
end

declare extern type PlayerDataService extends Instance with
	LoadFailureBehavior: EnumPlayerDataLoadFailureBehavior
end

declare extern type PlayerEmulatorService extends Instance with
end

declare extern type PlayerGui extends BasePlayerGui with
	CurrentScreenOrientation: EnumScreenOrientation
	ScreenOrientation: EnumScreenOrientation
	SelectionImageObject: Instance?
end

declare extern type PlayerHydrationService extends Instance with
end

declare extern type PlayerListConfiguration extends BaseCoreGuiConfiguration with
	Open: boolean
end

declare extern type PlayerMouse extends Mouse with
end

declare extern type PlayerScripts extends Instance with
end

declare extern type PlayerViewService extends Instance with
end

declare extern type Players extends Instance with
	BubbleChat: boolean
	CharacterAutoLoads: boolean
	ClassicChat: boolean
	LocalPlayer: Instance?
	MaxPlayers: number
	NumPlayers: number
	PreferredPlayers: number
	RespawnTime: number
	localPlayer: Instance?
	numPlayers: number
end

declare extern type Plugin extends Instance with
	CollisionEnabled: boolean
	GridSize: number
end

declare extern type PluginAction extends Instance with
	ActionId: string
	AllowBinding: boolean
	StatusTip: string
	Text: string
end

declare extern type PluginCapabilities extends Instance with
end

declare extern type PluginConnection extends Object with
	Connected: boolean
	TargetId: string
	Type: EnumPluginConnectionTargetType
end

declare extern type PluginConnectionService extends Instance with
end

declare extern type PluginDebugService extends Instance with
end

declare extern type PluginDragEvent extends Instance with
	Data: string
	MimeType: string
	Position: Vector2
	Sender: string
end

declare extern type PluginGuiService extends Instance with
end

declare extern type PluginManagementService extends Instance with
end

declare extern type PluginManager extends Instance with
end

declare extern type PluginManagerInterface extends Instance with
end

declare extern type PluginMenu extends Instance with
	Icon: string
	Title: string
end

declare extern type PluginMouse extends Mouse with
end

declare extern type PluginPolicyService extends Instance with
end

declare extern type PluginToolbar extends Instance with
end

declare extern type PluginToolbarButton extends Instance with
	ClickableWhenViewportHidden: boolean
	Enabled: boolean
	Icon: string
	IconContent: Content
end

declare extern type PointLight extends Light with
	Range: number
end

declare extern type PointsService extends Instance with
end

declare extern type PolicyService extends Instance with
end

declare extern type Pose extends PoseBase with
	CFrame: CFrame
	MaskWeight: number
end

declare extern type Preloaded extends Instance with
end

declare extern type PrismaticConstraint extends SlidingBallConstraint with
end

declare extern type ProceduralBehaviorSchedulerService extends Instance with
end

declare extern type ProceduralModel extends Model with
	GenerationError: string
	Generator: Instance?
	Size: Vector3
end

declare extern type ProcessInstancePhysicsService extends Instance with
end

declare extern type ProximityPrompt extends Instance with
	ActionText: string
	AutoLocalize: boolean
	ClickablePrompt: boolean
	Enabled: boolean
	Exclusivity: EnumProximityPromptExclusivity
	GamepadKeyCode: EnumKeyCode
	HoldDuration: number
	KeyboardKeyCode: EnumKeyCode
	MaxActivationDistance: number
	MaxIndicatorDistance: number
	ObjectText: string
	RequiresLineOfSight: boolean
	RootLocalizationTable: Instance?
	Style: EnumProximityPromptStyle
	UIOffset: Vector2
end

declare extern type ProximityPromptService extends Instance with
	Enabled: boolean
	MaxIndicatorsVisible: number
	MaxPromptsVisible: number
end

declare extern type PublishService extends Instance with
end

declare extern type PyramidHandleAdornment extends HandleAdornment with
	Height: number
	Shading: EnumAdornShading
	Sides: number
	Size: number
end

declare extern type QWidgetPluginGui extends PluginGui with
end

declare extern type RTAnimationTracker extends Instance with
	Active: boolean
	EnableFallbackAudioInput: boolean
	SessionName: string
	TrackerMode: EnumTrackerMode
	TrackerType: EnumTrackerType
end

declare extern type RayValue extends ValueBase with
	Value: Ray
end

declare extern type RbxAnalyticsService extends Instance with
end

declare extern type RealtimeMedia extends Instance with
	IsConnected: boolean
end

declare extern type RecommendationPages extends Pages with
end

declare extern type RecommendationService extends Instance with
end

declare extern type ReflectionMetadata extends Instance with
end

declare extern type ReflectionMetadataCallbacks extends Instance with
end

declare extern type ReflectionMetadataItem extends Instance with
	Browsable: boolean
	ClassCategory: string
	ClientOnly: boolean
	Constraint: string
	Deprecated: boolean
	EditingDisabled: boolean
	EditorType: string
	FFlag: string
	IsBackend: boolean
	PropertyOrder: number
	ScriptContext: string
	ServerOnly: boolean
	SliderScaling: string
	UIMaximum: number
	UIMinimum: number
	UINumTicks: number
end

declare extern type ReflectionMetadataClass extends ReflectionMetadataItem with
	ExplorerImageIndex: number
	ExplorerOrder: number
	Insertable: boolean
	PreferredParent: string
end

declare extern type ReflectionMetadataClasses extends Instance with
end

declare extern type ReflectionMetadataEnum extends ReflectionMetadataItem with
end

declare extern type ReflectionMetadataEnumItem extends ReflectionMetadataItem with
end

declare extern type ReflectionMetadataEnums extends Instance with
end

declare extern type ReflectionMetadataEvents extends Instance with
end

declare extern type ReflectionMetadataFunctions extends Instance with
end

declare extern type ReflectionMetadataMember extends ReflectionMetadataItem with
end

declare extern type ReflectionMetadataProperties extends Instance with
end

declare extern type ReflectionMetadataYieldFunctions extends Instance with
end

declare extern type ReflectionService extends Instance with
end

declare extern type RelativeGui extends GuiObject with
end

declare extern type RemoteCommandService extends Instance with
end

declare extern type RemoteCursorService extends Instance with
end

declare extern type RemoteDebuggerServer extends Instance with
end

declare extern type RemoteEvent extends BaseRemoteEvent with
end

declare extern type RemoteFunction extends Instance with
end

declare extern type RenderSettings extends Instance with
	AutoFRMLevel: number
	EagerBulkExecution: boolean
	EditQualityLevel: EnumQualityLevel
	EnableFRM: boolean
	ExportMergeByMaterial: boolean
	FrameRateManager: EnumFramerateManagerMode
	GraphicsMode: EnumGraphicsMode
	MeshCacheSize: number
	MeshPartDetailLevel: EnumMeshPartDetailLevel
	QualityLevel: EnumQualityLevel
	ReloadAssets: boolean
	RenderCSGTrianglesDebug: boolean
	ShowBoundingBoxes: boolean
	ViewMode: EnumViewMode
end

declare extern type RenderingTest extends Instance with
	CFrame: CFrame
	ComparisonDiffThreshold: number
	ComparisonMethod: EnumRenderingTestComparisonMethod
	ComparisonPsnrThreshold: number
	Description: string
	FieldOfView: number
	Orientation: Vector3
	PerfTest: boolean
	Position: Vector3
	QualityAuto: boolean
	QualityLevel: number
	RenderingTestFrameCount: number
	ShouldSkip: boolean
	Ticket: string
	Timeout: number
end

declare extern type ReplicatedFirst extends Instance with
end

declare extern type ReplicatedStorage extends Instance with
end

declare extern type ReverbSoundEffect extends SoundEffect with
	DecayTime: number
	Density: number
	Diffusion: number
	DryLevel: number
	WetLevel: number
end

declare extern type RibbonNotificationService extends Instance with
end

declare extern type RigidConstraint extends Constraint with
end

declare extern type RobloxPluginGuiService extends Instance with
end

declare extern type RobloxReplicatedStorage extends Instance with
end

declare extern type RobloxSerializableInstance extends Instance with
end

declare extern type RobloxServerStorage extends Instance with
end

declare extern type RocketPropulsion extends BodyMover with
	CartoonFactor: number
	MaxSpeed: number
	MaxThrust: number
	MaxTorque: Vector3
	Target: Instance?
	TargetOffset: Vector3
	TargetRadius: number
	ThrustD: number
	ThrustP: number
	TurnD: number
	TurnP: number
end

declare extern type RodConstraint extends Constraint with
	CurrentDistance: number
	Length: number
	LimitAngle0: number
	LimitAngle1: number
	LimitsEnabled: boolean
	Thickness: number
end

declare extern type RolloutValidation extends Instance with
end

declare extern type RolloutValidationService extends Instance with
end

declare extern type RomarkRbxAnalyticsService extends Instance with
end

declare extern type RomarkService extends Instance with
end

declare extern type RootImportData extends BaseImportData with
	AddModelToInventory: boolean
	Anchored: boolean
	AnimationIdForRestPose: number
	ExistingPackageId: string
	FileDimensions: Vector3
	ImportAsModelAsset: boolean
	ImportAsPackage: boolean
	InsertInWorkspace: boolean
	InsertWithScenePosition: boolean
	InvertNegativeFaces: boolean
	KeepZeroInfluenceBones: boolean
	MergeMeshes: boolean
	PhysicalConstraintType: EnumPhysicalConstraintType
	PolygonCount: number
	PreferredUploadId: number
	RestPose: EnumRestPose
	RigScale: EnumRigScale
	RigType: EnumRigType
	RigVisualization: boolean
	ScaleFactor: number
	ScaleUnit: EnumMeshScaleUnit
	UseSceneOriginAsPivot: boolean
	UsesCages: boolean
	ValidateUgcBody: boolean
	VersionedAssetId: number
	WorldForward: EnumNormalId
	WorldUp: EnumNormalId
end

declare extern type RopeConstraint extends Constraint with
	CurrentDistance: number
	Length: number
	Restitution: number
	Thickness: number
	WinchEnabled: boolean
	WinchForce: number
	WinchResponsiveness: number
	WinchSpeed: number
	WinchTarget: number
end

declare extern type Rotate extends JointInstance with
end

declare extern type RotateP extends DynamicRotate with
end

declare extern type RotateV extends DynamicRotate with
end

declare extern type RotationCurve extends Instance with
	Length: number
end

declare extern type RtMessagingService extends Instance with
end

declare extern type RunService extends Instance with
	FrameNumber: number
	RunState: EnumRunState
end

declare extern type StatsItem extends Instance with
	DisplayName: string
end

declare extern type RunningAverageItemDouble extends StatsItem with
end

declare extern type RunningAverageItemInt extends StatsItem with
end

declare extern type RunningAverageTimeIntervalItem extends StatsItem with
end

declare extern type RuntimeContentService extends Instance with
end

declare extern type RuntimeScriptService extends Instance with
end

declare extern type SafetyService extends Instance with
end

declare extern type SceneAnalysisService extends Instance with
end

declare extern type ScreenshotCapture extends Capture with
end

declare extern type ScreenshotHud extends Instance with
	CameraButtonIcon: string
	CameraButtonIconContent: Content
	CameraButtonPosition: UDim2
	CloseButtonPosition: UDim2
	CloseWhenScreenshotTaken: boolean
	ExperienceNameOverlayEnabled: boolean
	HideCoreGuiForCaptures: boolean
	HidePlayerGuiForCaptures: boolean
	OverlayFont: EnumFont
	UsernameOverlayEnabled: boolean
	Visible: boolean
end

declare extern type ScriptBuilder extends Instance with
end

declare extern type ScriptChangeService extends Instance with
end

declare extern type ScriptCloneWatcher extends Instance with
end

declare extern type ScriptCloneWatcherHelper extends Instance with
end

declare extern type ScriptCommitService extends Instance with
end

declare extern type ScriptContext extends Instance with
end

declare extern type ScriptDebugger extends Instance with
	CurrentLine: number
	IsDebugging: boolean
	IsPaused: boolean
	Script: Instance?
end

declare extern type ScriptDebuggerService extends Instance with
end

declare extern type ScriptDocument extends Instance with
end

declare extern type ScriptEditorService extends Instance with
end

declare extern type ScriptProfilerService extends Instance with
end

declare extern type ScriptRegistrationService extends Instance with
end

declare extern type ScriptRuntime extends Instance with
end

declare extern type ScriptService extends Instance with
end

declare extern type ScrollingFrame extends GuiObject with
	AbsoluteCanvasSize: Vector2
	AbsoluteWindowSize: Vector2
	AutomaticCanvasSize: EnumAutomaticSize
	BottomImage: string
	BottomImageContent: Content
	CanvasPosition: Vector2
	CanvasSize: UDim2
	ElasticBehavior: EnumElasticBehavior
	HorizontalScrollBarInset: EnumScrollBarInset
	MidImage: string
	MidImageContent: Content
	ScrollBarImageColor3: Color3
	ScrollBarImageTransparency: number
	ScrollBarThickness: number
	ScrollingDirection: EnumScrollingDirection
	ScrollingEnabled: boolean
	TopImage: string
	TopImageContent: Content
	VerticalScrollBarInset: EnumScrollBarInset
	VerticalScrollBarPosition: EnumVerticalScrollBarPosition
end

declare extern type Seat extends Part with
	Disabled: boolean
	Occupant: Instance?
end

declare extern type Selection extends Instance with
	SelectionThickness: number
end

declare extern type SelectionBox extends InstanceAdornment with
	LineThickness: number
	SurfaceColor: BrickColor
	SurfaceColor3: Color3
	SurfaceTransparency: number
end

declare extern type SelectionHighlightManager extends Instance with
end

declare extern type SelectionLasso extends GuiBase3d with
	Humanoid: Instance?
end

declare extern type SelectionPartLasso extends SelectionLasso with
	Part: Instance?
end

declare extern type SelectionPointLasso extends SelectionLasso with
	Point: Vector3
end

declare extern type SelectionSphere extends PVAdornment with
	SurfaceColor: BrickColor
	SurfaceColor3: Color3
	SurfaceTransparency: number
end

declare extern type SelfViewConfiguration extends BaseCoreGuiConfiguration with
	Open: boolean
end

declare extern type SerializationService extends Instance with
end

declare extern type ServerReplicator extends NetworkReplicator with
end

declare extern type ServerScriptService extends Instance with
end

declare extern type ServerStorage extends Instance with
end

declare extern type ServiceVisibilityService extends Instance with
end

declare extern type SessionCheckService extends Instance with
end

declare extern type SessionService extends Instance with
end

declare extern type SharedTableRegistry extends Instance with
end

declare extern type Shirt extends Clothing with
	ShirtTemplate: string
	ShirtTemplateContent: Content
end

declare extern type ShirtGraphic extends CharacterAppearance with
	Color3: Color3
	Graphic: string
	TextureContent: Content
end

declare extern type SkateboardController extends Controller with
	Steer: number
	Throttle: number
end

declare extern type SkateboardPlatform extends Part with
	Controller: Instance?
	ControllingHumanoid: Instance?
	Steer: number
	StickyWheels: boolean
	Throttle: number
end

declare extern type Skin extends CharacterAppearance with
	SkinColor: BrickColor
end

declare extern type Sky extends Instance with
	CelestialBodiesShown: boolean
	MoonAngularSize: number
	MoonTextureContent: Content
	MoonTextureId: string
	SkyboxBackContent: Content
	SkyboxBk: string
	SkyboxDn: string
	SkyboxDownContent: Content
	SkyboxFrontContent: Content
	SkyboxFt: string
	SkyboxLeftContent: Content
	SkyboxLf: string
	SkyboxOrientation: Vector3
	SkyboxRightContent: Content
	SkyboxRt: string
	SkyboxUp: string
	SkyboxUpContent: Content
	StarCount: number
	SunAngularSize: number
	SunTextureContent: Content
	SunTextureId: string
end

declare extern type SlimAnimationDataEntity extends Instance with
end

declare extern type SlimAnimationReplicationService extends Instance with
end

declare extern type SlimContentProvider extends CacheableContentProvider with
end

declare extern type SlimDebugSettings extends Instance with
end

declare extern type SlimReplicationService extends Instance with
end

declare extern type SlimService extends Instance with
end

declare extern type Smoke extends Instance with
	Color: Color3
	Enabled: boolean
	LocalTransparencyModifier: number
	Opacity: number
	RiseVelocity: number
	Size: number
	TimeScale: number
end

declare extern type SmoothVoxelsUpgraderService extends Instance with
end

declare extern type Snap extends JointInstance with
end

declare extern type SnippetService extends Instance with
end

declare extern type SocialService extends Instance with
end

declare extern type SolidModelContentProvider extends CacheableContentProvider with
end

declare extern type Sound extends Instance with
	AcousticSimulationEnabled: boolean
	AudioContent: Content
	EmitterSize: number
	IsLoaded: boolean
	IsPaused: boolean
	IsPlaying: boolean
	LoopRegion: NumberRange
	Looped: boolean
	MaxDistance: number
	MinDistance: number
	Pitch: number
	PlayOnRemove: boolean
	PlaybackLoudness: number
	PlaybackRegion: NumberRange
	PlaybackRegionsEnabled: boolean
	PlaybackSpeed: number
	Playing: boolean
	RollOffMaxDistance: number
	RollOffMinDistance: number
	RollOffMode: EnumRollOffMode
	SoundGroup: Instance?
	SoundId: string
	TimeLength: number
	TimePosition: number
	Volume: number
	isPlaying: boolean
end

declare extern type SoundGroup extends Instance with
	Volume: number
end

declare extern type SoundService extends Instance with
	AcousticSimulationEnabled: boolean
	AmbientReverb: EnumReverbType
	CharacterSoundsUseNewApi: EnumRolloutState
	DefaultListenerLocation: EnumListenerLocation
	DiffractionEnabled: boolean
	DistanceFactor: number
	DopplerScale: number
	ListenerCFrame: CFrame
	ListenerObject: Instance?
	ListenerType: EnumListenerType
	OcclusionEnabled: boolean
	RespectFilteringEnabled: boolean
	ReverbEnabled: boolean
	RolloffScale: number
end

declare extern type SoundShimService extends Instance with
end

declare extern type Sparkles extends Instance with
	Color: Color3
	Enabled: boolean
	LocalTransparencyModifier: number
	SparkleColor: Color3
	TimeScale: number
end

declare extern type SpawnLocation extends Part with
	AllowTeamChangeOnTouch: boolean
	Duration: number
	Enabled: boolean
	Neutral: boolean
	TeamColor: BrickColor
end

declare extern type SpawnerService extends Instance with
end

declare extern type SpecialMesh extends FileMesh with
	MeshType: EnumMeshType
end

declare extern type SphereHandleAdornment extends HandleAdornment with
	Radius: number
	Shading: EnumAdornShading
end

declare extern type SpotLight extends Light with
	Angle: number
	Face: EnumNormalId
	Range: number
end

declare extern type SpringConstraint extends Constraint with
	Coils: number
	CurrentLength: number
	Damping: number
	FreeLength: number
	LimitsEnabled: boolean
	MaxForce: number
	MaxLength: number
	MinLength: number
	Radius: number
	Stiffness: number
	Thickness: number
end

declare extern type StackFrame extends Instance with
end

declare extern type StandalonePluginScripts extends Instance with
end

declare extern type StandardPages extends Pages with
end

declare extern type StartPageService extends Instance with
end

declare extern type StarterPlayerScripts extends Instance with
end

declare extern type StarterCharacterScripts extends StarterPlayerScripts with
end

declare extern type StarterGear extends Instance with
end

declare extern type StarterGui extends BasePlayerGui with
	ProcessUserInput: boolean
	ResetPlayerGuiOnSpawn: boolean
	ScreenOrientation: EnumScreenOrientation
	ShowDevelopmentGui: boolean
end

declare extern type StarterPack extends Instance with
end

declare extern type StarterPlayer extends Instance with
	AllowCustomAnimations: boolean
	AutoJumpEnabled: boolean
	CameraMaxZoomDistance: number
	CameraMinZoomDistance: number
	CameraMode: EnumCameraMode
	CharacterBreakJointsOnDeath: boolean
	CharacterJumpHeight: number
	CharacterJumpPower: number
	CharacterMaxSlopeAngle: number
	CharacterUseJumpPower: boolean
	CharacterWalkSpeed: number
	ClassicDeath: boolean
	DevCameraOcclusionMode: EnumDevCameraOcclusionMode
	DevComputerCameraMovementMode: EnumDevComputerCameraMovementMode
	DevComputerMovementMode: EnumDevComputerMovementMode
	DevTouchCameraMovementMode: EnumDevTouchCameraMovementMode
	DevTouchMovementMode: EnumDevTouchMovementMode
	EnableMouseLockOption: boolean
	HealthDisplayDistance: number
	LoadCharacterAppearance: boolean
	LuaCharacterController: EnumCharacterControlMode
	NameDisplayDistance: number
	UserEmotesEnabled: boolean
end

declare extern type StartupMessageService extends Instance with
end

declare extern type Stats extends Instance with
	ContactsCount: number
	DataReceiveKbps: number
	DataSendKbps: number
	FrameTime: number
	HeartbeatTime: number
	HeartbeatTimeMs: number
	InstanceCount: number
	MemoryTrackingEnabled: boolean
	MovingPrimitivesCount: number
	PhysicsReceiveKbps: number
	PhysicsSendKbps: number
	PhysicsStepTime: number
	PhysicsStepTimeMs: number
	PrimitivesCount: number
	RenderCPUFrameTime: number
	RenderGPUFrameTime: number
	SceneDrawcallCount: number
	SceneTriangleCount: number
	ShadowsDrawcallCount: number
	ShadowsTriangleCount: number
	UI2DDrawcallCount: number
	UI2DTriangleCount: number
	UI3DDrawcallCount: number
	UI3DTriangleCount: number
end

declare extern type Status extends Model with
end

declare extern type StopWatchReporter extends Instance with
end

declare extern type StringValue extends ValueBase with
	Value: string
end

declare extern type Studio extends Instance with
	ActionOnStopSync: EnumActionOnStopSync
	CommandBarLocalState: boolean
	DeprecatedObjectsShown: boolean
	HintColor: Color3
	InformationColor: Color3
	LuaDebuggerEnabled: boolean
	LuaDebuggerEnabledAtStartup: boolean
	PermissionLevelShown: EnumPermissionLevelShown
	PluginDebuggingEnabled: boolean
	PreferredTextSize: EnumPreferredTextSize
	Rulers: string
	RuntimeUndoBehavior: EnumRuntimeUndoBehavior
	ScriptTimeoutLength: number
	Theme: Instance?
	TypeColor: Color3
end

declare extern type StudioActionOverride extends Object with
end

declare extern type StudioAssetService extends Instance with
end

declare extern type StudioAttachment extends Instance with
	AutoHideParent: boolean
	IsArrowVisible: boolean
	Offset: Vector2
	SourceAnchorPoint: Vector2
	TargetAnchorPoint: Vector2
end

declare extern type StudioCallout extends Instance with
end

declare extern type StudioCameraService extends Instance with
end

declare extern type StudioCaptureService extends Instance with
end

declare extern type StudioData extends Instance with
end

declare extern type StudioDeviceEmulatorService extends Instance with
end

declare extern type StudioDeviceSimulatorService extends Instance with
end

declare extern type StudioObjectBase extends Instance with
end

declare extern type StudioPublishService extends Instance with
end

declare extern type StudioScreenshotCapture extends Instance with
	BufferFormat: EnumStudioCaptureScreenshotFormat
	BufferStatus: EnumStudioCaptureBufferStatus
	OriginalSize: Vector2
	Position: Vector2
	Resolution: Vector2
	UICaptureMode: EnumUICaptureMode
end

declare extern type StudioScriptDebugEventListener extends Instance with
end

declare extern type StudioSdkService extends Instance with
end

declare extern type StudioService extends Instance with
	ActiveScript: Instance?
	DraggerSolveConstraints: boolean
	DrawConstraintsOnTop: boolean
	GridSize: number
	RotateIncrement: number
	ShowConstraintDetails: boolean
	StudioLocaleId: string
	UseLocalSpace: boolean
end

declare extern type StudioTestService extends Instance with
	EditModeActive: boolean
end

declare extern type StudioTheme extends Instance with
end

declare extern type StudioUserService extends Instance with
end

declare extern type StudioWidget extends StudioObjectBase with
end

declare extern type StudioWidgetsService extends Instance with
end

declare extern type StyleBase extends Instance with
end

declare extern type StyleDerive extends Instance with
	Priority: number
	StyleSheet: Instance?
end

declare extern type StyleLink extends Instance with
	StyleSheet: Instance?
end

declare extern type StyleQuery extends Instance with
	IsActive: boolean
end

declare extern type StyleRule extends StyleBase with
	Priority: number
	Selector: string
	SelectorError: string
end

declare extern type StyleSheet extends StyleBase with
end

declare extern type StylingService extends Instance with
end

declare extern type SunRaysEffect extends PostEffect with
	Intensity: number
	Spread: number
end

declare extern type SurfaceAppearance extends Instance with
	AlphaMode: EnumAlphaMode
	Color: Color3
	ColorMap: string
	ColorMapContent: Content
	EmissiveMaskContent: Content
	EmissiveStrength: number
	EmissiveTint: Color3
	MetalnessMap: string
	MetalnessMapContent: Content
	NormalMap: string
	NormalMapContent: Content
	ResampleMode: EnumResamplerMode
	RoughnessMap: string
	RoughnessMapContent: Content
end

declare extern type SurfaceGui extends SurfaceGuiBase with
	AlwaysOnTop: boolean
	Brightness: number
	CanvasSize: Vector2
	ClipsDescendants: boolean
	LightInfluence: number
	MaxDistance: number
	PixelsPerStud: number
	SizingMode: EnumSurfaceGuiSizingMode
	ToolPunchThroughDistance: number
	ZOffset: number
end

declare extern type SurfaceLight extends Light with
	Angle: number
	Face: EnumNormalId
	Range: number
end

declare extern type SurfaceSelection extends PartAdornment with
	TargetSurface: EnumNormalId
end

declare extern type SwimController extends ControllerBase with
	AccelerationTime: number
	PitchMaxTorque: number
	PitchSpeedFactor: number
	RollMaxTorque: number
	RollSpeedFactor: number
end

declare extern type SyncScriptBuilder extends ScriptBuilder with
	CompileTarget: EnumCompileTarget
	CoverageInfo: boolean
	DebugInfo: boolean
	PackAsSource: boolean
	RawBytecode: boolean
end

declare extern type SystemThemeService extends Instance with
end

declare extern type TaskScheduler extends Instance with
	SchedulerDutyCycle: number
	SchedulerRate: number
	ThreadPoolConfig: EnumThreadPoolConfig
	ThreadPoolSize: number
end

declare extern type Team extends Instance with
	AutoAssignable: boolean
	AutoColorCharacters: boolean
	Score: number
	TeamColor: BrickColor
end

declare extern type TeamCreateData extends Instance with
end

declare extern type TeamCreatePublishService extends Instance with
end

declare extern type TeamCreateService extends Instance with
end

declare extern type Teams extends Instance with
end

declare extern type TelemetryService extends Instance with
end

declare extern type TeleportAsyncResult extends Instance with
	PrivateServerId: string
	ReservedServerAccessCode: string
end

declare extern type TeleportOptions extends Instance with
	ReservedServerAccessCode: string
	ServerInstanceId: string
	ShouldReserveServer: boolean
end

declare extern type TeleportService extends Instance with
	CustomizedTeleportUI: boolean
end

declare extern type TemporaryCageMeshProvider extends Instance with
end

declare extern type TemporaryScriptService extends Instance with
end

declare extern type Terrain extends BasePart with
	IsSmooth: boolean
	MaxExtents: Region3int16
	WaterColor: Color3
	WaterReflectance: number
	WaterTransparency: number
	WaterWaveSize: number
	WaterWaveSpeed: number
end

declare extern type TerrainDetail extends Instance with
	ColorMap: string
	ColorMapContent: Content
	EmissiveMaskContent: Content
	EmissiveStrength: number
	EmissiveTint: Color3
	Face: EnumTerrainFace
	MaterialPattern: EnumMaterialPattern
	MetalnessMap: string
	MetalnessMapContent: Content
	NormalMap: string
	NormalMapContent: Content
	RoughnessMap: string
	RoughnessMapContent: Content
	StudsPerTile: number
end

declare extern type TerrainIterateOperation extends Object with
end

declare extern type TerrainModifyOperation extends Object with
end

declare extern type TerrainReadOperation extends Object with
end

declare extern type TerrainRegion extends Instance with
	IsSmooth: boolean
	SizeInCells: Vector3
end

declare extern type TerrainWriteOperation extends Object with
end

declare extern type TestCase extends Instance with
end

declare extern type TestService extends Instance with
	AutoRuns: boolean
	Description: string
	ErrorCount: number
	ExecuteWithStudioRun: boolean
	Is30FpsThrottleEnabled: boolean
	IsPhysicsEnvironmentalThrottled: boolean
	IsSleepAllowed: boolean
	NumberOfPlayers: number
	SimulateSecondsLag: number
	TestCount: number
	ThrottlePhysicsToRealtime: boolean
	Timeout: number
	WarnCount: number
end

declare extern type TextBox extends GuiObject with
	ClearTextOnFocus: boolean
	ContentText: string
	CursorPosition: number
	Font: EnumFont
	FontFace: Font
	FontSize: EnumFontSize
	LineHeight: number
	MaxVisibleGraphemes: number
	MultiLine: boolean
	OpenTypeFeatures: string
	OpenTypeFeaturesError: string
	PlaceholderColor3: Color3
	PlaceholderText: string
	RichText: boolean
	SelectionStart: number
	ShowNativeInput: boolean
	Text: string
	TextBounds: Vector2
	TextColor: BrickColor
	TextColor3: Color3
	TextDirection: EnumTextDirection
	TextEditable: boolean
	TextFits: boolean
	TextScaled: boolean
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
	TextTransparency: number
	TextTruncate: EnumTextTruncate
	TextWrap: boolean
	TextWrapped: boolean
	TextXAlignment: EnumTextXAlignment
	TextYAlignment: EnumTextYAlignment
end

declare extern type TextBoxService extends Instance with
end

declare extern type TextButton extends GuiButton with
	ContentText: string
	Font: EnumFont
	FontFace: Font
	FontSize: EnumFontSize
	LineHeight: number
	LocalizedText: string
	MaxVisibleGraphemes: number
	OpenTypeFeatures: string
	OpenTypeFeaturesError: string
	RichText: boolean
	Text: string
	TextBounds: Vector2
	TextColor: BrickColor
	TextColor3: Color3
	TextDirection: EnumTextDirection
	TextFits: boolean
	TextScaled: boolean
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
	TextTransparency: number
	TextTruncate: EnumTextTruncate
	TextWrap: boolean
	TextWrapped: boolean
	TextXAlignment: EnumTextXAlignment
	TextYAlignment: EnumTextYAlignment
end

declare extern type TextChannel extends Instance with
	DirectChatRequester: Instance?
end

declare extern type TextChannelWindow extends GuiObject with
	Target: Instance?
end

declare extern type TextChatCommand extends Instance with
	AutocompleteVisible: boolean
	Enabled: boolean
	PrimaryAlias: string
	SecondaryAlias: string
end

declare extern type TextChatMessage extends Instance with
	BubbleChatMessageProperties: Instance?
	ChatWindowMessageProperties: Instance?
	MessageId: string
	Metadata: string
	PrefixText: string
	Status: EnumTextChatMessageStatus
	Text: string
	TextChannel: Instance?
	TextSource: Instance?
	Translation: string
end

declare extern type TextChatService extends Instance with
	ChatTranslationEnabled: boolean
	ChatVersion: EnumChatVersion
	CreateDefaultCommands: boolean
	CreateDefaultTextChannels: boolean
end

declare extern type TextFilterResult extends Instance with
end

declare extern type TextFilterTranslatedResult extends Instance with
	SourceLanguage: string
	SourceText: Instance?
end

declare extern type TextGenerator extends Instance with
	Seed: number
	SystemPrompt: string
	Temperature: number
	TopP: number
end

declare extern type TextLabel extends GuiLabel with
	ContentText: string
	Font: EnumFont
	FontFace: Font
	FontSize: EnumFontSize
	LineHeight: number
	LocalizedText: string
	MaxVisibleGraphemes: number
	OpenTypeFeatures: string
	OpenTypeFeaturesError: string
	RichText: boolean
	Text: string
	TextBounds: Vector2
	TextColor: BrickColor
	TextColor3: Color3
	TextDirection: EnumTextDirection
	TextFits: boolean
	TextScaled: boolean
	TextSize: number
	TextStrokeColor3: Color3
	TextStrokeTransparency: number
	TextTransparency: number
	TextTruncate: EnumTextTruncate
	TextWrap: boolean
	TextWrapped: boolean
	TextXAlignment: EnumTextXAlignment
	TextYAlignment: EnumTextYAlignment
end

declare extern type TextService extends Instance with
end

declare extern type TextSource extends Instance with
	CanSend: boolean
	UserId: number
end

declare extern type Texture extends Decal with
	OffsetStudsU: number
	OffsetStudsV: number
	StudsPerTileU: number
	StudsPerTileV: number
end

declare extern type TextureGenerationPartGroup extends Instance with
end

declare extern type TextureGenerationService extends Instance with
end

declare extern type TextureGenerationUnwrappingRequest extends Instance with
end

declare extern type ThirdPartyUserService extends Instance with
end

declare extern type ThreadState extends Instance with
end

declare extern type TimerService extends Instance with
end

declare extern type ToastNotificationService extends Instance with
end

declare extern type Torque extends Constraint with
	RelativeTo: EnumActuatorRelativeTo
	Torque: Vector3
end

declare extern type TorsionSpringConstraint extends Constraint with
	Coils: number
	CurrentAngle: number
	Damping: number
	LimitEnabled: boolean
	LimitsEnabled: boolean
	MaxAngle: number
	MaxTorque: number
	Radius: number
	Restitution: number
	Stiffness: number
end

declare extern type TotalCountTimeIntervalItem extends StatsItem with
end

declare extern type TouchInputService extends Instance with
end

declare extern type TouchTransmitter extends Instance with
end

declare extern type TraceRouteService extends Instance with
end

declare extern type TracerService extends Instance with
end

declare extern type TrackerLodController extends Instance with
	AudioMode: EnumTrackerLodFlagMode
	VideoExtrapolationMode: EnumTrackerExtrapolationFlagMode
	VideoLodMode: EnumTrackerLodValueMode
	VideoMode: EnumTrackerLodFlagMode
end

declare extern type TrackerStreamAnimation extends Instance with
end

declare extern type Trail extends Instance with
	Attachment0: Instance?
	Attachment1: Instance?
	Brightness: number
	Color: ColorSequence
	Enabled: boolean
	FaceCamera: boolean
	Lifetime: number
	LightEmission: number
	LightInfluence: number
	LocalTransparencyModifier: number
	MaxLength: number
	MinLength: number
	Texture: string
	TextureContent: Content
	TextureLength: number
	TextureMode: EnumTextureMode
	Transparency: NumberSequence
	WidthScale: NumberSequence
end

declare extern type Translator extends Instance with
	LocaleId: string
end

declare extern type TremoloSoundEffect extends SoundEffect with
	Depth: number
	Duty: number
	Frequency: number
end

declare extern type TrussPart extends BasePart with
	Style: EnumStyle
end

declare extern type TutorialService extends Instance with
end

declare extern type TweenBase extends Instance with
	PlaybackState: EnumPlaybackState
end

declare extern type Tween extends TweenBase with
	Instance: Instance?
end

declare extern type TweenService extends Instance with
end

declare extern type UGCAvatarService extends Instance with
end

declare extern type UGCValidationService extends Instance with
end

declare extern type UIBase extends Instance with
end

declare extern type UIComponent extends UIBase with
end

declare extern type UIConstraint extends UIComponent with
end

declare extern type UIAspectRatioConstraint extends UIConstraint with
	AspectRatio: number
	AspectType: EnumAspectType
	DominantAxis: EnumDominantAxis
end

declare extern type UICorner extends UIComponent with
	BottomLeftRadius: UDim
	BottomRightRadius: UDim
	CornerRadius: UDim
	TopLeftRadius: UDim
	TopRightRadius: UDim
end

declare extern type UIDragDetector extends UIComponent with
	ActivatedCursorIcon: string
	ActivatedCursorIconContent: Content
	BoundingBehavior: EnumUIDragDetectorBoundingBehavior
	BoundingUI: Instance?
	CursorIcon: string
	CursorIconContent: Content
	DragAxis: Vector2
	DragRelativity: EnumUIDragDetectorDragRelativity
	DragRotation: number
	DragSpace: EnumUIDragDetectorDragSpace
	DragStyle: EnumUIDragDetectorDragStyle
	DragUDim2: UDim2
	Enabled: boolean
	MaxDragAngle: number
	MaxDragTranslation: UDim2
	MinDragAngle: number
	MinDragTranslation: UDim2
	ReferenceUIInstance: Instance?
	ResponseStyle: EnumUIDragDetectorResponseStyle
	SelectionModeDragSpeed: UDim2
	SelectionModeRotateSpeed: number
	UIDragSpeedAxisMapping: EnumUIDragSpeedAxisMapping
end

declare extern type UIDragDetectorService extends Instance with
end

declare extern type UIFlexItem extends UIComponent with
	FlexMode: EnumUIFlexMode
	GrowRatio: number
	ItemLineAlignment: EnumItemLineAlignment
	ShrinkRatio: number
end

declare extern type UIGradient extends UIComponent with
	Color: ColorSequence
	Enabled: boolean
	Offset: Vector2
	Rotation: number
	Scale: number
	TileMode: EnumGradientTileMode
	Transparency: NumberSequence
	Type: EnumGradientType
end

declare extern type UILayout extends UIComponent with
end

declare extern type UIGridStyleLayout extends UILayout with
	AbsoluteContentSize: Vector2
	FillDirection: EnumFillDirection
	HorizontalAlignment: EnumHorizontalAlignment
	SortOrder: EnumSortOrder
	VerticalAlignment: EnumVerticalAlignment
end

declare extern type UIGridLayout extends UIGridStyleLayout with
	AbsoluteCellCount: Vector2
	AbsoluteCellSize: Vector2
	CellPadding: UDim2
	CellSize: UDim2
	FillDirectionMaxCells: number
	StartCorner: EnumStartCorner
end

declare extern type UIListLayout extends UIGridStyleLayout with
	HorizontalFlex: EnumUIFlexAlignment
	ItemLineAlignment: EnumItemLineAlignment
	Padding: UDim
	VerticalFlex: EnumUIFlexAlignment
	Wraps: boolean
end

declare extern type UIPadding extends UIComponent with
	PaddingBottom: UDim
	PaddingLeft: UDim
	PaddingRight: UDim
	PaddingTop: UDim
end

declare extern type UIPageLayout extends UIGridStyleLayout with
	Animated: boolean
	Circular: boolean
	CurrentPage: Instance?
	EasingDirection: EnumEasingDirection
	EasingStyle: EnumEasingStyle
	GamepadInputEnabled: boolean
	Padding: UDim
	ScrollWheelInputEnabled: boolean
	TouchInputEnabled: boolean
	TweenTime: number
end

declare extern type UIScale extends UIComponent with
	Scale: number
end

declare extern type UIShadow extends UIComponent with
	BlurRadius: UDim
	Color: Color3
	Enabled: boolean
	Offset: UDim2
	Spread: UDim2
	Transparency: number
	ZIndex: number
end

declare extern type UISizeConstraint extends UIConstraint with
	MaxSize: Vector2
	MinSize: Vector2
end

declare extern type UIStroke extends UIComponent with
	ApplyStrokeMode: EnumApplyStrokeMode
	BorderOffset: UDim
	BorderStrokePosition: EnumBorderStrokePosition
	Color: Color3
	Enabled: boolean
	LineJoinMode: EnumLineJoinMode
	StrokeSizingMode: EnumStrokeSizingMode
	Thickness: number
	Transparency: number
	ZIndex: number
end

declare extern type UITableLayout extends UIGridStyleLayout with
	FillEmptySpaceColumns: boolean
	FillEmptySpaceRows: boolean
	MajorAxis: EnumTableMajorAxis
	Padding: UDim2
end

declare extern type UITextSizeConstraint extends UIConstraint with
	MaxTextSize: number
	MinTextSize: number
end

declare extern type UnionOperation extends PartOperation with
end

declare extern type UniqueIdLookupService extends Instance with
end

declare extern type UniversalConstraint extends Constraint with
	LimitsEnabled: boolean
	MaxAngle: number
	Radius: number
	Restitution: number
end

declare extern type UnreliableRemoteEvent extends BaseRemoteEvent with
end

declare extern type UnvalidatedAssetService extends Instance with
end

declare extern type UserGameSettings extends Instance with
	ComputerCameraMovementMode: EnumComputerCameraMovementMode
	ComputerMovementMode: EnumComputerMovementMode
	ControlMode: EnumControlMode
	GamepadCameraSensitivity: number
	MouseSensitivity: number
	RCCProfilerRecordFrameRate: number
	RCCProfilerRecordTimeFrame: number
	RotationType: EnumRotationType
	SavedQualityLevel: EnumSavedQualitySetting
	TouchCameraMovementMode: EnumTouchCameraMovementMode
	TouchMovementMode: EnumTouchMovementMode
	VRSmoothRotationEnabled: boolean
	VignetteEnabled: boolean
end

declare extern type UserInputService extends Instance with
	AccelerometerEnabled: boolean
	GamepadEnabled: boolean
	GyroscopeEnabled: boolean
	KeyboardEnabled: boolean
	ModalEnabled: boolean
	MouseBehavior: EnumMouseBehavior
	MouseDeltaSensitivity: number
	MouseEnabled: boolean
	MouseIcon: string
	MouseIconContent: Content
	MouseIconEnabled: boolean
	OnScreenKeyboardPosition: Vector2
	OnScreenKeyboardSize: Vector2
	OnScreenKeyboardVisible: boolean
	PreferredInput: EnumPreferredInput
	TouchEnabled: boolean
	UserHeadCFrame: CFrame
	VREnabled: boolean
end

declare extern type UserService extends Instance with
end

declare extern type UserSettings extends GenericSettings with
end

declare extern type UserStorageService extends LocalStorageService with
end

declare extern type VRService extends Instance with
	AutomaticScaling: EnumVRScaling
	AvatarGestures: boolean
	ControllerModels: EnumVRControllerModelMode
	FadeOutViewOnCollision: boolean
	GuiInputUserCFrame: EnumUserCFrame
	LaserPointer: EnumVRLaserPointerMode
	ThirdPersonFollowCamEnabled: boolean
	VREnabled: boolean
end

declare extern type VRStatusService extends Instance with
end

declare extern type ValueCurve extends Instance with
	Length: number
	ValueType: string
end

declare extern type Vector3Curve extends Instance with
end

declare extern type Vector3Value extends ValueBase with
	Value: Vector3
end

declare extern type VectorForce extends Constraint with
	ApplyAtCenterOfMass: boolean
	Force: Vector3
	RelativeTo: EnumActuatorRelativeTo
end

declare extern type VehicleController extends Controller with
end

declare extern type VehicleSeat extends BasePart with
	AreHingesDetected: number
	Disabled: boolean
	HeadsUpDisplay: boolean
	MaxSpeed: number
	Occupant: Instance?
	Steer: number
	SteerFloat: number
	Throttle: number
	ThrottleFloat: number
	Torque: number
	TurnSpeed: number
end

declare extern type VelocityMotor extends JointInstance with
	CurrentAngle: number
	DesiredAngle: number
	Hole: Instance?
	MaxVelocity: number
end

declare extern type VersionControlService extends Instance with
end

declare extern type VideoCapture extends Capture with
	TimeLength: number
end

declare extern type VideoCaptureService extends Instance with
end

declare extern type VideoDeviceInput extends Instance with
	Active: boolean
	CameraId: string
	CaptureQuality: EnumVideoDeviceCaptureQuality
	IsReady: boolean
end

declare extern type VideoDisplay extends GuiObject with
	ResampleMode: EnumResamplerMode
	ScaleType: EnumScaleType
	TileSize: UDim2
	VideoColor3: Color3
	VideoRectOffset: Vector2
	VideoRectSize: Vector2
	VideoTransparency: number
end

declare extern type VideoFrame extends GuiObject with
	InternalVideoUsage: EnumInternalVideoUsage
	IsLoaded: boolean
	Looped: boolean
	MaximumResolution: EnumVideoSampleSize
	Playing: boolean
	Resolution: Vector2
	RollOffMaxDistance: number
	RollOffMinDistance: number
	RollOffMode: EnumRollOffMode
	TimeLength: number
	TimePosition: number
	Video: string
	VideoContent: Content
	Volume: number
end

declare extern type VideoPlayer extends Instance with
	InternalVideoUsage: EnumInternalVideoUsage
	IsLoaded: boolean
	IsPlaying: boolean
	Looping: boolean
	MaximumResolution: EnumVideoSampleSize
	PlaybackSpeed: number
	Resolution: Vector2
	TimeLength: number
	TimePosition: number
	VideoContent: Content
	Volume: number
end

declare extern type VideoSampler extends Object with
	TimeLength: number
	VideoContent: Content
end

declare extern type VideoScreenCaptureService extends Instance with
end

declare extern type VideoService extends Instance with
end

declare extern type ViewportFrame extends GuiObject with
	Ambient: Color3
	CurrentCamera: Instance?
	ImageColor3: Color3
	ImageTransparency: number
	LightColor: Color3
	LightDirection: Vector3
end

declare extern type VirtualInput extends Object with
end

declare extern type VirtualInputManager extends Instance with
end

declare extern type VirtualUser extends Instance with
end

declare extern type VisibilityCheckDispatcher extends Instance with
end

declare extern type Visit extends Instance with
end

declare extern type VisualizationMode extends Instance with
end

declare extern type VisualizationModeCategory extends Instance with
end

declare extern type VisualizationModeService extends Instance with
end

declare extern type VoiceChatInternal extends Instance with
	VoiceChatState: EnumVoiceChatState
end

declare extern type VoiceChatService extends Instance with
	DefaultDistanceAttenuation: EnumVoiceChatDistanceAttenuationType
	EnableDefaultVoice: boolean
	UseAudioApi: EnumAudioApiRollout
end

declare extern type VoxelBuffer extends Object with
end

declare extern type WebSocketClient extends Instance with
	ConnectionState: EnumWebSocketState
end

declare extern type WebSocketService extends Instance with
end

declare extern type WebStreamClient extends Object with
	ConnectionState: EnumWebStreamClientState
end

declare extern type WebViewService extends Instance with
end

declare extern type WedgePart extends FormFactorPart with
end

declare extern type Weld extends JointInstance with
end

declare extern type WeldConstraint extends Instance with
	Active: boolean
	Enabled: boolean
	Part0: Instance?
	Part1: Instance?
end

declare extern type Wire extends Instance with
	Connected: boolean
	SourceInstance: Instance?
	SourceName: string
	TargetInstance: Instance?
	TargetName: string
end

declare extern type WireframeHandleAdornment extends HandleAdornment with
	Scale: Vector3
	Thickness: number
end

declare extern type WorldRoot extends Model with
end

declare extern type Workspace extends WorldRoot with
	AirDensity: number
	AirTurbulenceIntensity: number
	AllowThirdPartySales: boolean
	ClientAnimatorThrottling: EnumClientAnimatorThrottlingMode
	CurrentCamera: Instance?
	DistributedGameTime: number
	FallHeightEnabled: boolean
	FallenPartsDestroyHeight: number
	FilteringEnabled: boolean
	GlobalWind: Vector3
	Gravity: number
	InsertPoint: Vector3
	InterpolationThrottling: EnumInterpolationThrottlingMode
	LuauTypeCheckMode: EnumLuauTypeCheckMode
	Retargeting: EnumAnimatorRetargetingMode
	StreamingEnabled: boolean
	Terrain: Instance?
end

declare extern type WorkspaceAnnotation extends Annotation with
end

declare extern type WorldModel extends WorldRoot with
end

declare extern type WrapDeformMeshProvider extends Instance with
end

declare extern type WrapDeformer extends BaseWrap with
end

declare extern type WrapLayer extends BaseWrap with
	AutoSkin: EnumWrapLayerAutoSkin
	BindOffset: CFrame
	Enabled: boolean
	Order: number
	Puffiness: number
	ReferenceMeshContent: Content
	ReferenceMeshId: string
	ReferenceOrigin: CFrame
	ReferenceOriginWorld: CFrame
	ShrinkFactor: number
end

declare extern type WrapTarget extends BaseWrap with
	Stiffness: number
end

declare extern type WrapTextureTransfer extends Instance with
	ReferenceCageMeshContent: Content
	UVMaxBound: Vector2
	UVMinBound: Vector2
end

-- ── Enums ───────────────────────────────────────────────────────────────

declare extern type EnumAccessModifierType extends EnumItem with
end

declare extern type EnumAccessModifierType_INTERNAL extends Enum with
	Allow: EnumAccessModifierType
	Deny: EnumAccessModifierType
	function GetEnumItems(self): { EnumAccessModifierType }
	function FromName(self, name: string): EnumAccessModifierType?
	function FromValue(self, value: number): EnumAccessModifierType?
end

declare extern type EnumAccessoryType extends EnumItem with
end

declare extern type EnumAccessoryType_INTERNAL extends Enum with
	Unknown: EnumAccessoryType
	Hat: EnumAccessoryType
	Hair: EnumAccessoryType
	Face: EnumAccessoryType
	Neck: EnumAccessoryType
	Shoulder: EnumAccessoryType
	Front: EnumAccessoryType
	Back: EnumAccessoryType
	Waist: EnumAccessoryType
	TShirt: EnumAccessoryType
	Shirt: EnumAccessoryType
	Pants: EnumAccessoryType
	Jacket: EnumAccessoryType
	Sweater: EnumAccessoryType
	Shorts: EnumAccessoryType
	LeftShoe: EnumAccessoryType
	RightShoe: EnumAccessoryType
	DressSkirt: EnumAccessoryType
	Eyebrow: EnumAccessoryType
	Eyelash: EnumAccessoryType
	function GetEnumItems(self): { EnumAccessoryType }
	function FromName(self, name: string): EnumAccessoryType?
	function FromValue(self, value: number): EnumAccessoryType?
end

declare extern type EnumActionOnAutoResumeSync extends EnumItem with
end

declare extern type EnumActionOnAutoResumeSync_INTERNAL extends Enum with
	DontResume: EnumActionOnAutoResumeSync
	KeepStudio: EnumActionOnAutoResumeSync
	KeepLocal: EnumActionOnAutoResumeSync
	function GetEnumItems(self): { EnumActionOnAutoResumeSync }
	function FromName(self, name: string): EnumActionOnAutoResumeSync?
	function FromValue(self, value: number): EnumActionOnAutoResumeSync?
end

declare extern type EnumActionOnStopSync extends EnumItem with
end

declare extern type EnumActionOnStopSync_INTERNAL extends Enum with
	AlwaysAsk: EnumActionOnStopSync
	KeepLocalFiles: EnumActionOnStopSync
	DeleteLocalFiles: EnumActionOnStopSync
	function GetEnumItems(self): { EnumActionOnStopSync }
	function FromName(self, name: string): EnumActionOnStopSync?
	function FromValue(self, value: number): EnumActionOnStopSync?
end

declare extern type EnumActionType extends EnumItem with
end

declare extern type EnumActionType_INTERNAL extends Enum with
	Nothing: EnumActionType
	Pause: EnumActionType
	Lose: EnumActionType
	Draw: EnumActionType
	Win: EnumActionType
	function GetEnumItems(self): { EnumActionType }
	function FromName(self, name: string): EnumActionType?
	function FromValue(self, value: number): EnumActionType?
end

declare extern type EnumActivePayerStatus extends EnumItem with
end

declare extern type EnumActivePayerStatus_INTERNAL extends Enum with
	Unknown: EnumActivePayerStatus
	Never: EnumActivePayerStatus
	Lapsed: EnumActivePayerStatus
	Casual50Percent: EnumActivePayerStatus
	Intermediate35Percent: EnumActivePayerStatus
	Top15Percent: EnumActivePayerStatus
	function GetEnumItems(self): { EnumActivePayerStatus }
	function FromName(self, name: string): EnumActivePayerStatus?
	function FromValue(self, value: number): EnumActivePayerStatus?
end

declare extern type EnumActuatorRelativeTo extends EnumItem with
end

declare extern type EnumActuatorRelativeTo_INTERNAL extends Enum with
	Attachment0: EnumActuatorRelativeTo
	Attachment1: EnumActuatorRelativeTo
	World: EnumActuatorRelativeTo
	function GetEnumItems(self): { EnumActuatorRelativeTo }
	function FromName(self, name: string): EnumActuatorRelativeTo?
	function FromValue(self, value: number): EnumActuatorRelativeTo?
end

declare extern type EnumActuatorType extends EnumItem with
end

declare extern type EnumActuatorType_INTERNAL extends Enum with
	None: EnumActuatorType
	Motor: EnumActuatorType
	Servo: EnumActuatorType
	function GetEnumItems(self): { EnumActuatorType }
	function FromName(self, name: string): EnumActuatorType?
	function FromValue(self, value: number): EnumActuatorType?
end

declare extern type EnumAdAvailabilityResult extends EnumItem with
end

declare extern type EnumAdAvailabilityResult_INTERNAL extends Enum with
	IsAvailable: EnumAdAvailabilityResult
	DeviceIneligible: EnumAdAvailabilityResult
	ExperienceIneligible: EnumAdAvailabilityResult
	InternalError: EnumAdAvailabilityResult
	NoFill: EnumAdAvailabilityResult
	PlayerIneligible: EnumAdAvailabilityResult
	PublisherIneligible: EnumAdAvailabilityResult
	function GetEnumItems(self): { EnumAdAvailabilityResult }
	function FromName(self, name: string): EnumAdAvailabilityResult?
	function FromValue(self, value: number): EnumAdAvailabilityResult?
end

declare extern type EnumAdEventType extends EnumItem with
end

declare extern type EnumAdEventType_INTERNAL extends Enum with
	VideoLoaded: EnumAdEventType
	VideoRemoved: EnumAdEventType
	UserCompletedVideo: EnumAdEventType
	RewardedAdLoaded: EnumAdEventType
	RewardedAdGrant: EnumAdEventType
	RewardedAdUnloaded: EnumAdEventType
	function GetEnumItems(self): { EnumAdEventType }
	function FromName(self, name: string): EnumAdEventType?
	function FromValue(self, value: number): EnumAdEventType?
end

declare extern type EnumAdFormat extends EnumItem with
end

declare extern type EnumAdFormat_INTERNAL extends Enum with
	RewardedVideo: EnumAdFormat
	function GetEnumItems(self): { EnumAdFormat }
	function FromName(self, name: string): EnumAdFormat?
	function FromValue(self, value: number): EnumAdFormat?
end

declare extern type EnumAdShape extends EnumItem with
end

declare extern type EnumAdShape_INTERNAL extends Enum with
	HorizontalRectangle: EnumAdShape
	function GetEnumItems(self): { EnumAdShape }
	function FromName(self, name: string): EnumAdShape?
	function FromValue(self, value: number): EnumAdShape?
end

declare extern type EnumAdTeleportMethod extends EnumItem with
end

declare extern type EnumAdTeleportMethod_INTERNAL extends Enum with
	Undefined: EnumAdTeleportMethod
	PortalForward: EnumAdTeleportMethod
	InGameMenuBackButton: EnumAdTeleportMethod
	UIBackButton: EnumAdTeleportMethod
	function GetEnumItems(self): { EnumAdTeleportMethod }
	function FromName(self, name: string): EnumAdTeleportMethod?
	function FromValue(self, value: number): EnumAdTeleportMethod?
end

declare extern type EnumAdUIEventType extends EnumItem with
end

declare extern type EnumAdUIEventType_INTERNAL extends Enum with
	AdLabelClicked: EnumAdUIEventType
	VolumeButtonClicked: EnumAdUIEventType
	FullscreenButtonClicked: EnumAdUIEventType
	PlayButtonClicked: EnumAdUIEventType
	PauseButtonClicked: EnumAdUIEventType
	CloseButtonClicked: EnumAdUIEventType
	WhyThisAdClicked: EnumAdUIEventType
	PlayEventTriggered: EnumAdUIEventType
	PauseEventTriggered: EnumAdUIEventType
	function GetEnumItems(self): { EnumAdUIEventType }
	function FromName(self, name: string): EnumAdUIEventType?
	function FromValue(self, value: number): EnumAdUIEventType?
end

declare extern type EnumAdUIType extends EnumItem with
end

declare extern type EnumAdUIType_INTERNAL extends Enum with
	None: EnumAdUIType
	Image: EnumAdUIType
	Video: EnumAdUIType
	function GetEnumItems(self): { EnumAdUIType }
	function FromName(self, name: string): EnumAdUIType?
	function FromValue(self, value: number): EnumAdUIType?
end

declare extern type EnumAdUnitStatus extends EnumItem with
end

declare extern type EnumAdUnitStatus_INTERNAL extends Enum with
	Inactive: EnumAdUnitStatus
	Active: EnumAdUnitStatus
	function GetEnumItems(self): { EnumAdUnitStatus }
	function FromName(self, name: string): EnumAdUnitStatus?
	function FromValue(self, value: number): EnumAdUnitStatus?
end

declare extern type EnumAdornCullingMode extends EnumItem with
end

declare extern type EnumAdornCullingMode_INTERNAL extends Enum with
	Automatic: EnumAdornCullingMode
	Never: EnumAdornCullingMode
	function GetEnumItems(self): { EnumAdornCullingMode }
	function FromName(self, name: string): EnumAdornCullingMode?
	function FromValue(self, value: number): EnumAdornCullingMode?
end

declare extern type EnumAdornShading extends EnumItem with
end

declare extern type EnumAdornShading_INTERNAL extends Enum with
	Default: EnumAdornShading
	Shaded: EnumAdornShading
	XRay: EnumAdornShading
	XRayShaded: EnumAdornShading
	AlwaysOnTop: EnumAdornShading
	function GetEnumItems(self): { EnumAdornShading }
	function FromName(self, name: string): EnumAdornShading?
	function FromValue(self, value: number): EnumAdornShading?
end

declare extern type EnumAgeCheckStatus extends EnumItem with
end

declare extern type EnumAgeCheckStatus_INTERNAL extends Enum with
	Unchecked: EnumAgeCheckStatus
	Checked: EnumAgeCheckStatus
	function GetEnumItems(self): { EnumAgeCheckStatus }
	function FromName(self, name: string): EnumAgeCheckStatus?
	function FromValue(self, value: number): EnumAgeCheckStatus?
end

declare extern type EnumAlignType extends EnumItem with
end

declare extern type EnumAlignType_INTERNAL extends Enum with
	Parallel: EnumAlignType
	Perpendicular: EnumAlignType
	PrimaryAxisParallel: EnumAlignType
	PrimaryAxisPerpendicular: EnumAlignType
	PrimaryAxisLookAt: EnumAlignType
	AllAxes: EnumAlignType
	function GetEnumItems(self): { EnumAlignType }
	function FromName(self, name: string): EnumAlignType?
	function FromValue(self, value: number): EnumAlignType?
end

declare extern type EnumAlphaMode extends EnumItem with
end

declare extern type EnumAlphaMode_INTERNAL extends Enum with
	Overlay: EnumAlphaMode
	Transparency: EnumAlphaMode
	TintMask: EnumAlphaMode
	Opaque: EnumAlphaMode
	function GetEnumItems(self): { EnumAlphaMode }
	function FromName(self, name: string): EnumAlphaMode?
	function FromValue(self, value: number): EnumAlphaMode?
end

declare extern type EnumAnalyticsCustomFieldKeys extends EnumItem with
end

declare extern type EnumAnalyticsCustomFieldKeys_INTERNAL extends Enum with
	CustomField01: EnumAnalyticsCustomFieldKeys
	CustomField02: EnumAnalyticsCustomFieldKeys
	CustomField03: EnumAnalyticsCustomFieldKeys
	function GetEnumItems(self): { EnumAnalyticsCustomFieldKeys }
	function FromName(self, name: string): EnumAnalyticsCustomFieldKeys?
	function FromValue(self, value: number): EnumAnalyticsCustomFieldKeys?
end

declare extern type EnumAnalyticsEconomyAction extends EnumItem with
end

declare extern type EnumAnalyticsEconomyAction_INTERNAL extends Enum with
	Default: EnumAnalyticsEconomyAction
	Acquire: EnumAnalyticsEconomyAction
	Spend: EnumAnalyticsEconomyAction
	function GetEnumItems(self): { EnumAnalyticsEconomyAction }
	function FromName(self, name: string): EnumAnalyticsEconomyAction?
	function FromValue(self, value: number): EnumAnalyticsEconomyAction?
end

declare extern type EnumAnalyticsEconomyFlowType extends EnumItem with
end

declare extern type EnumAnalyticsEconomyFlowType_INTERNAL extends Enum with
	Sink: EnumAnalyticsEconomyFlowType
	Source: EnumAnalyticsEconomyFlowType
	function GetEnumItems(self): { EnumAnalyticsEconomyFlowType }
	function FromName(self, name: string): EnumAnalyticsEconomyFlowType?
	function FromValue(self, value: number): EnumAnalyticsEconomyFlowType?
end

declare extern type EnumAnalyticsEconomyTransactionType extends EnumItem with
end

declare extern type EnumAnalyticsEconomyTransactionType_INTERNAL extends Enum with
	IAP: EnumAnalyticsEconomyTransactionType
	Shop: EnumAnalyticsEconomyTransactionType
	Gameplay: EnumAnalyticsEconomyTransactionType
	ContextualPurchase: EnumAnalyticsEconomyTransactionType
	TimedReward: EnumAnalyticsEconomyTransactionType
	Onboarding: EnumAnalyticsEconomyTransactionType
	function GetEnumItems(self): { EnumAnalyticsEconomyTransactionType }
	function FromName(self, name: string): EnumAnalyticsEconomyTransactionType?
	function FromValue(self, value: number): EnumAnalyticsEconomyTransactionType?
end

declare extern type EnumAnalyticsLogLevel extends EnumItem with
end

declare extern type EnumAnalyticsLogLevel_INTERNAL extends Enum with
	Trace: EnumAnalyticsLogLevel
	Debug: EnumAnalyticsLogLevel
	Information: EnumAnalyticsLogLevel
	Warning: EnumAnalyticsLogLevel
	Error: EnumAnalyticsLogLevel
	Fatal: EnumAnalyticsLogLevel
	function GetEnumItems(self): { EnumAnalyticsLogLevel }
	function FromName(self, name: string): EnumAnalyticsLogLevel?
	function FromValue(self, value: number): EnumAnalyticsLogLevel?
end

declare extern type EnumAnalyticsProgressionStatus extends EnumItem with
end

declare extern type EnumAnalyticsProgressionStatus_INTERNAL extends Enum with
	Default: EnumAnalyticsProgressionStatus
	Begin: EnumAnalyticsProgressionStatus
	Complete: EnumAnalyticsProgressionStatus
	Abandon: EnumAnalyticsProgressionStatus
	Fail: EnumAnalyticsProgressionStatus
	function GetEnumItems(self): { EnumAnalyticsProgressionStatus }
	function FromName(self, name: string): EnumAnalyticsProgressionStatus?
	function FromValue(self, value: number): EnumAnalyticsProgressionStatus?
end

declare extern type EnumAnalyticsProgressionType extends EnumItem with
end

declare extern type EnumAnalyticsProgressionType_INTERNAL extends Enum with
	Custom: EnumAnalyticsProgressionType
	Start: EnumAnalyticsProgressionType
	Fail: EnumAnalyticsProgressionType
	Complete: EnumAnalyticsProgressionType
	function GetEnumItems(self): { EnumAnalyticsProgressionType }
	function FromName(self, name: string): EnumAnalyticsProgressionType?
	function FromValue(self, value: number): EnumAnalyticsProgressionType?
end

declare extern type EnumAnimationClipFromVideoStatus extends EnumItem with
end

declare extern type EnumAnimationClipFromVideoStatus_INTERNAL extends Enum with
	Initializing: EnumAnimationClipFromVideoStatus
	Pending: EnumAnimationClipFromVideoStatus
	Processing: EnumAnimationClipFromVideoStatus
	ErrorGeneric: EnumAnimationClipFromVideoStatus
	Success: EnumAnimationClipFromVideoStatus
	ErrorVideoTooLong: EnumAnimationClipFromVideoStatus
	ErrorNoPersonDetected: EnumAnimationClipFromVideoStatus
	ErrorVideoUnstable: EnumAnimationClipFromVideoStatus
	Timeout: EnumAnimationClipFromVideoStatus
	Cancelled: EnumAnimationClipFromVideoStatus
	ErrorMultiplePeople: EnumAnimationClipFromVideoStatus
	ErrorUploadingVideo: EnumAnimationClipFromVideoStatus
	function GetEnumItems(self): { EnumAnimationClipFromVideoStatus }
	function FromName(self, name: string): EnumAnimationClipFromVideoStatus?
	function FromValue(self, value: number): EnumAnimationClipFromVideoStatus?
end

declare extern type EnumAnimationNodeBlend2DInputMode extends EnumItem with
end

declare extern type EnumAnimationNodeBlend2DInputMode_INTERNAL extends Enum with
	Cartesian: EnumAnimationNodeBlend2DInputMode
	Polar: EnumAnimationNodeBlend2DInputMode
	function GetEnumItems(self): { EnumAnimationNodeBlend2DInputMode }
	function FromName(self, name: string): EnumAnimationNodeBlend2DInputMode?
	function FromValue(self, value: number): EnumAnimationNodeBlend2DInputMode?
end

declare extern type EnumAnimationNodeInterruptible extends EnumItem with
end

declare extern type EnumAnimationNodeInterruptible_INTERNAL extends Enum with
	Always: EnumAnimationNodeInterruptible
	Finished: EnumAnimationNodeInterruptible
	Trigger: EnumAnimationNodeInterruptible
	function GetEnumItems(self): { EnumAnimationNodeInterruptible }
	function FromName(self, name: string): EnumAnimationNodeInterruptible?
	function FromValue(self, value: number): EnumAnimationNodeInterruptible?
end

declare extern type EnumAnimationNodePhaseSync extends EnumItem with
end

declare extern type EnumAnimationNodePhaseSync_INTERNAL extends Enum with
	Synced: EnumAnimationNodePhaseSync
	Unsynced: EnumAnimationNodePhaseSync
	function GetEnumItems(self): { EnumAnimationNodePhaseSync }
	function FromName(self, name: string): EnumAnimationNodePhaseSync?
	function FromValue(self, value: number): EnumAnimationNodePhaseSync?
end

declare extern type EnumAnimationNodePlayMode extends EnumItem with
end

declare extern type EnumAnimationNodePlayMode_INTERNAL extends Enum with
	Loop: EnumAnimationNodePlayMode
	PingPong: EnumAnimationNodePlayMode
	OnceAndHold: EnumAnimationNodePlayMode
	OnceAndReset: EnumAnimationNodePlayMode
	function GetEnumItems(self): { EnumAnimationNodePlayMode }
	function FromName(self, name: string): EnumAnimationNodePlayMode?
	function FromValue(self, value: number): EnumAnimationNodePlayMode?
end

declare extern type EnumAnimationNodeTransitionType extends EnumItem with
end

declare extern type EnumAnimationNodeTransitionType_INTERNAL extends Enum with
	CrossFade: EnumAnimationNodeTransitionType
	InertialBlend: EnumAnimationNodeTransitionType
	DeadBlend: EnumAnimationNodeTransitionType
	function GetEnumItems(self): { EnumAnimationNodeTransitionType }
	function FromName(self, name: string): EnumAnimationNodeTransitionType?
	function FromValue(self, value: number): EnumAnimationNodeTransitionType?
end

declare extern type EnumAnimationNodeType extends EnumItem with
end

declare extern type EnumAnimationNodeType_INTERNAL extends Enum with
	InvalidNode: EnumAnimationNodeType
	AddNode: EnumAnimationNodeType
	OverNode: EnumAnimationNodeType
	Blend1DNode: EnumAnimationNodeType
	Blend2DNode: EnumAnimationNodeType
	ClipNode: EnumAnimationNodeType
	GraphOutput: EnumAnimationNodeType
	MaskNode: EnumAnimationNodeType
	PrioritySelectNode: EnumAnimationNodeType
	RandomSequenceNode: EnumAnimationNodeType
	SelectNode: EnumAnimationNodeType
	SequenceNode: EnumAnimationNodeType
	SpeedNode: EnumAnimationNodeType
	SubtractNode: EnumAnimationNodeType
	function GetEnumItems(self): { EnumAnimationNodeType }
	function FromName(self, name: string): EnumAnimationNodeType?
	function FromValue(self, value: number): EnumAnimationNodeType?
end

declare extern type EnumAnimationNodeWaitFor extends EnumItem with
end

declare extern type EnumAnimationNodeWaitFor_INTERNAL extends Enum with
	Finished: EnumAnimationNodeWaitFor
	Trigger: EnumAnimationNodeWaitFor
	function GetEnumItems(self): { EnumAnimationNodeWaitFor }
	function FromName(self, name: string): EnumAnimationNodeWaitFor?
	function FromValue(self, value: number): EnumAnimationNodeWaitFor?
end

declare extern type EnumAnimationPriority extends EnumItem with
end

declare extern type EnumAnimationPriority_INTERNAL extends Enum with
	Idle: EnumAnimationPriority
	Movement: EnumAnimationPriority
	Action: EnumAnimationPriority
	Action2: EnumAnimationPriority
	Action3: EnumAnimationPriority
	Action4: EnumAnimationPriority
	Core: EnumAnimationPriority
	function GetEnumItems(self): { EnumAnimationPriority }
	function FromName(self, name: string): EnumAnimationPriority?
	function FromValue(self, value: number): EnumAnimationPriority?
end

declare extern type EnumAnimatorRetargetingMode extends EnumItem with
end

declare extern type EnumAnimatorRetargetingMode_INTERNAL extends Enum with
	Default: EnumAnimatorRetargetingMode
	Disabled: EnumAnimatorRetargetingMode
	Enabled: EnumAnimatorRetargetingMode
	function GetEnumItems(self): { EnumAnimatorRetargetingMode }
	function FromName(self, name: string): EnumAnimatorRetargetingMode?
	function FromValue(self, value: number): EnumAnimatorRetargetingMode?
end

declare extern type EnumAnnotationChannelContentPreference extends EnumItem with
end

declare extern type EnumAnnotationChannelContentPreference_INTERNAL extends Enum with
	None: EnumAnnotationChannelContentPreference
	All: EnumAnnotationChannelContentPreference
	Unknown: EnumAnnotationChannelContentPreference
	function GetEnumItems(self): { EnumAnnotationChannelContentPreference }
	function FromName(self, name: string): EnumAnnotationChannelContentPreference?
	function FromValue(self, value: number): EnumAnnotationChannelContentPreference?
end

declare extern type EnumAnnotationEditingMode extends EnumItem with
end

declare extern type EnumAnnotationEditingMode_INTERNAL extends Enum with
	None: EnumAnnotationEditingMode
	PlacingNew: EnumAnnotationEditingMode
	WritingNew: EnumAnnotationEditingMode
	function GetEnumItems(self): { EnumAnnotationEditingMode }
	function FromName(self, name: string): EnumAnnotationEditingMode?
	function FromValue(self, value: number): EnumAnnotationEditingMode?
end

declare extern type EnumAnnotationPlaceContentPreference extends EnumItem with
end

declare extern type EnumAnnotationPlaceContentPreference_INTERNAL extends Enum with
	None: EnumAnnotationPlaceContentPreference
	All: EnumAnnotationPlaceContentPreference
	MentionsAndReplies: EnumAnnotationPlaceContentPreference
	Unknown: EnumAnnotationPlaceContentPreference
	function GetEnumItems(self): { EnumAnnotationPlaceContentPreference }
	function FromName(self, name: string): EnumAnnotationPlaceContentPreference?
	function FromValue(self, value: number): EnumAnnotationPlaceContentPreference?
end

declare extern type EnumAnnotationRequestStatus extends EnumItem with
end

declare extern type EnumAnnotationRequestStatus_INTERNAL extends Enum with
	Success: EnumAnnotationRequestStatus
	Loading: EnumAnnotationRequestStatus
	ErrorInternalFailure: EnumAnnotationRequestStatus
	ErrorNotFound: EnumAnnotationRequestStatus
	ErrorModerated: EnumAnnotationRequestStatus
	function GetEnumItems(self): { EnumAnnotationRequestStatus }
	function FromName(self, name: string): EnumAnnotationRequestStatus?
	function FromValue(self, value: number): EnumAnnotationRequestStatus?
end

declare extern type EnumAnnotationRequestType extends EnumItem with
end

declare extern type EnumAnnotationRequestType_INTERNAL extends Enum with
	Unknown: EnumAnnotationRequestType
	Create: EnumAnnotationRequestType
	Resolve: EnumAnnotationRequestType
	Delete: EnumAnnotationRequestType
	Edit: EnumAnnotationRequestType
	function GetEnumItems(self): { EnumAnnotationRequestType }
	function FromName(self, name: string): EnumAnnotationRequestType?
	function FromValue(self, value: number): EnumAnnotationRequestType?
end

declare extern type EnumAntiAliasing extends EnumItem with
end

declare extern type EnumAntiAliasing_INTERNAL extends Enum with
	Disabled: EnumAntiAliasing
	Enabled: EnumAntiAliasing
	function GetEnumItems(self): { EnumAntiAliasing }
	function FromName(self, name: string): EnumAntiAliasing?
	function FromValue(self, value: number): EnumAntiAliasing?
end

declare extern type EnumAppLifecycleManagerState extends EnumItem with
end

declare extern type EnumAppLifecycleManagerState_INTERNAL extends Enum with
	Detached: EnumAppLifecycleManagerState
	Active: EnumAppLifecycleManagerState
	Inactive: EnumAppLifecycleManagerState
	Hidden: EnumAppLifecycleManagerState
	function GetEnumItems(self): { EnumAppLifecycleManagerState }
	function FromName(self, name: string): EnumAppLifecycleManagerState?
	function FromValue(self, value: number): EnumAppLifecycleManagerState?
end

declare extern type EnumAppShellActionType extends EnumItem with
end

declare extern type EnumAppShellActionType_INTERNAL extends Enum with
	None: EnumAppShellActionType
	OpenApp: EnumAppShellActionType
	TapChatTab: EnumAppShellActionType
	TapConversationEntry: EnumAppShellActionType
	TapAvatarTab: EnumAppShellActionType
	ReadConversation: EnumAppShellActionType
	TapGamePageTab: EnumAppShellActionType
	TapHomePageTab: EnumAppShellActionType
	GamePageLoaded: EnumAppShellActionType
	HomePageLoaded: EnumAppShellActionType
	AvatarEditorPageLoaded: EnumAppShellActionType
	HomePageInteractive: EnumAppShellActionType
	function GetEnumItems(self): { EnumAppShellActionType }
	function FromName(self, name: string): EnumAppShellActionType?
	function FromValue(self, value: number): EnumAppShellActionType?
end

declare extern type EnumAppShellFeature extends EnumItem with
end

declare extern type EnumAppShellFeature_INTERNAL extends Enum with
	None: EnumAppShellFeature
	Chat: EnumAppShellFeature
	AvatarEditor: EnumAppShellFeature
	GamePage: EnumAppShellFeature
	HomePage: EnumAppShellFeature
	More: EnumAppShellFeature
	Landing: EnumAppShellFeature
	WatchPage: EnumAppShellFeature
	function GetEnumItems(self): { EnumAppShellFeature }
	function FromName(self, name: string): EnumAppShellFeature?
	function FromValue(self, value: number): EnumAppShellFeature?
end

declare extern type EnumAppUpdateStatus extends EnumItem with
end

declare extern type EnumAppUpdateStatus_INTERNAL extends Enum with
	Unknown: EnumAppUpdateStatus
	NotSupported: EnumAppUpdateStatus
	Failed: EnumAppUpdateStatus
	NotAvailable: EnumAppUpdateStatus
	Available: EnumAppUpdateStatus
	AvailableBoundChannel: EnumAppUpdateStatus
	AvailableBetaProgram: EnumAppUpdateStatus
	function GetEnumItems(self): { EnumAppUpdateStatus }
	function FromName(self, name: string): EnumAppUpdateStatus?
	function FromValue(self, value: number): EnumAppUpdateStatus?
end

declare extern type EnumApplyStrokeMode extends EnumItem with
end

declare extern type EnumApplyStrokeMode_INTERNAL extends Enum with
	Contextual: EnumApplyStrokeMode
	Border: EnumApplyStrokeMode
	function GetEnumItems(self): { EnumApplyStrokeMode }
	function FromName(self, name: string): EnumApplyStrokeMode?
	function FromValue(self, value: number): EnumApplyStrokeMode?
end

declare extern type EnumAspectType extends EnumItem with
end

declare extern type EnumAspectType_INTERNAL extends Enum with
	FitWithinMaxSize: EnumAspectType
	ScaleWithParentSize: EnumAspectType
	function GetEnumItems(self): { EnumAspectType }
	function FromName(self, name: string): EnumAspectType?
	function FromValue(self, value: number): EnumAspectType?
end

declare extern type EnumAssetCreatorType extends EnumItem with
end

declare extern type EnumAssetCreatorType_INTERNAL extends Enum with
	User: EnumAssetCreatorType
	Group: EnumAssetCreatorType
	function GetEnumItems(self): { EnumAssetCreatorType }
	function FromName(self, name: string): EnumAssetCreatorType?
	function FromValue(self, value: number): EnumAssetCreatorType?
end

declare extern type EnumAssetFetchStatus extends EnumItem with
end

declare extern type EnumAssetFetchStatus_INTERNAL extends Enum with
	Success: EnumAssetFetchStatus
	Failure: EnumAssetFetchStatus
	None: EnumAssetFetchStatus
	Loading: EnumAssetFetchStatus
	TimedOut: EnumAssetFetchStatus
	function GetEnumItems(self): { EnumAssetFetchStatus }
	function FromName(self, name: string): EnumAssetFetchStatus?
	function FromValue(self, value: number): EnumAssetFetchStatus?
end

declare extern type EnumAssetRepresentation extends EnumItem with
end

declare extern type EnumAssetRepresentation_INTERNAL extends Enum with
	FullLength: EnumAssetRepresentation
	ShortPreview: EnumAssetRepresentation
	function GetEnumItems(self): { EnumAssetRepresentation }
	function FromName(self, name: string): EnumAssetRepresentation?
	function FromValue(self, value: number): EnumAssetRepresentation?
end

declare extern type EnumAssetType extends EnumItem with
end

declare extern type EnumAssetType_INTERNAL extends Enum with
	Image: EnumAssetType
	TShirt: EnumAssetType
	Audio: EnumAssetType
	Mesh: EnumAssetType
	Lua: EnumAssetType
	Hat: EnumAssetType
	Place: EnumAssetType
	Model: EnumAssetType
	Shirt: EnumAssetType
	Pants: EnumAssetType
	Decal: EnumAssetType
	Head: EnumAssetType
	Face: EnumAssetType
	Gear: EnumAssetType
	Badge: EnumAssetType
	Animation: EnumAssetType
	Torso: EnumAssetType
	RightArm: EnumAssetType
	LeftArm: EnumAssetType
	LeftLeg: EnumAssetType
	RightLeg: EnumAssetType
	Package: EnumAssetType
	GamePass: EnumAssetType
	Plugin: EnumAssetType
	MeshPart: EnumAssetType
	HairAccessory: EnumAssetType
	FaceAccessory: EnumAssetType
	NeckAccessory: EnumAssetType
	ShoulderAccessory: EnumAssetType
	FrontAccessory: EnumAssetType
	BackAccessory: EnumAssetType
	WaistAccessory: EnumAssetType
	ClimbAnimation: EnumAssetType
	DeathAnimation: EnumAssetType
	FallAnimation: EnumAssetType
	IdleAnimation: EnumAssetType
	JumpAnimation: EnumAssetType
	RunAnimation: EnumAssetType
	SwimAnimation: EnumAssetType
	WalkAnimation: EnumAssetType
	PoseAnimation: EnumAssetType
	EarAccessory: EnumAssetType
	EyeAccessory: EnumAssetType
	EmoteAnimation: EnumAssetType
	Video: EnumAssetType
	TShirtAccessory: EnumAssetType
	ShirtAccessory: EnumAssetType
	PantsAccessory: EnumAssetType
	JacketAccessory: EnumAssetType
	SweaterAccessory: EnumAssetType
	ShortsAccessory: EnumAssetType
	LeftShoeAccessory: EnumAssetType
	RightShoeAccessory: EnumAssetType
	DressSkirtAccessory: EnumAssetType
	FontFamily: EnumAssetType
	EyebrowAccessory: EnumAssetType
	EyelashAccessory: EnumAssetType
	MoodAnimation: EnumAssetType
	DynamicHead: EnumAssetType
	FaceMakeup: EnumAssetType
	LipMakeup: EnumAssetType
	EyeMakeup: EnumAssetType
	VoxelFragment: EnumAssetType
	AvatarBackground: EnumAssetType
	TextDocument: EnumAssetType
	function GetEnumItems(self): { EnumAssetType }
	function FromName(self, name: string): EnumAssetType?
	function FromValue(self, value: number): EnumAssetType?
end

declare extern type EnumAssetTypeVerification extends EnumItem with
end

declare extern type EnumAssetTypeVerification_INTERNAL extends Enum with
	Default: EnumAssetTypeVerification
	ClientOnly: EnumAssetTypeVerification
	Always: EnumAssetTypeVerification
	function GetEnumItems(self): { EnumAssetTypeVerification }
	function FromName(self, name: string): EnumAssetTypeVerification?
	function FromValue(self, value: number): EnumAssetTypeVerification?
end

declare extern type EnumAudioApiRollout extends EnumItem with
end

declare extern type EnumAudioApiRollout_INTERNAL extends Enum with
	Disabled: EnumAudioApiRollout
	Automatic: EnumAudioApiRollout
	Enabled: EnumAudioApiRollout
	function GetEnumItems(self): { EnumAudioApiRollout }
	function FromName(self, name: string): EnumAudioApiRollout?
	function FromValue(self, value: number): EnumAudioApiRollout?
end

declare extern type EnumAudioCaptureMode extends EnumItem with
end

declare extern type EnumAudioCaptureMode_INTERNAL extends Enum with
	function GetEnumItems(self): { EnumAudioCaptureMode }
	function FromName(self, name: string): EnumAudioCaptureMode?
	function FromValue(self, value: number): EnumAudioCaptureMode?
end

declare extern type EnumAudioChannelLayout extends EnumItem with
end

declare extern type EnumAudioChannelLayout_INTERNAL extends Enum with
	Mono: EnumAudioChannelLayout
	Stereo: EnumAudioChannelLayout
	Quad: EnumAudioChannelLayout
	Surround_5: EnumAudioChannelLayout
	Surround_5_1: EnumAudioChannelLayout
	Surround_7_1: EnumAudioChannelLayout
	Surround_7_1_4: EnumAudioChannelLayout
	function GetEnumItems(self): { EnumAudioChannelLayout }
	function FromName(self, name: string): EnumAudioChannelLayout?
	function FromValue(self, value: number): EnumAudioChannelLayout?
end

declare extern type EnumAudioFilterType extends EnumItem with
end

declare extern type EnumAudioFilterType_INTERNAL extends Enum with
	Peak: EnumAudioFilterType
	LowShelf: EnumAudioFilterType
	HighShelf: EnumAudioFilterType
	Lowpass12dB: EnumAudioFilterType
	Lowpass24dB: EnumAudioFilterType
	Lowpass48dB: EnumAudioFilterType
	Highpass12dB: EnumAudioFilterType
	Highpass24dB: EnumAudioFilterType
	Highpass48dB: EnumAudioFilterType
	Bandpass: EnumAudioFilterType
	Notch: EnumAudioFilterType
	Lowpass6dB: EnumAudioFilterType
	function GetEnumItems(self): { EnumAudioFilterType }
	function FromName(self, name: string): EnumAudioFilterType?
	function FromValue(self, value: number): EnumAudioFilterType?
end

declare extern type EnumAudioSimulationFidelity extends EnumItem with
end

declare extern type EnumAudioSimulationFidelity_INTERNAL extends Enum with
	None: EnumAudioSimulationFidelity
	Automatic: EnumAudioSimulationFidelity
	function GetEnumItems(self): { EnumAudioSimulationFidelity }
	function FromName(self, name: string): EnumAudioSimulationFidelity?
	function FromValue(self, value: number): EnumAudioSimulationFidelity?
end

declare extern type EnumAudioSubType extends EnumItem with
end

declare extern type EnumAudioSubType_INTERNAL extends Enum with
	Music: EnumAudioSubType
	SoundEffect: EnumAudioSubType
	function GetEnumItems(self): { EnumAudioSubType }
	function FromName(self, name: string): EnumAudioSubType?
	function FromValue(self, value: number): EnumAudioSubType?
end

declare extern type EnumAudioWindowSize extends EnumItem with
end

declare extern type EnumAudioWindowSize_INTERNAL extends Enum with
	Small: EnumAudioWindowSize
	Medium: EnumAudioWindowSize
	Large: EnumAudioWindowSize
	function GetEnumItems(self): { EnumAudioWindowSize }
	function FromName(self, name: string): EnumAudioWindowSize?
	function FromValue(self, value: number): EnumAudioWindowSize?
end

declare extern type EnumAuthorityMode extends EnumItem with
end

declare extern type EnumAuthorityMode_INTERNAL extends Enum with
	Server: EnumAuthorityMode
	Automatic: EnumAuthorityMode
	function GetEnumItems(self): { EnumAuthorityMode }
	function FromName(self, name: string): EnumAuthorityMode?
	function FromValue(self, value: number): EnumAuthorityMode?
end

declare extern type EnumAutoIndentRule extends EnumItem with
end

declare extern type EnumAutoIndentRule_INTERNAL extends Enum with
	Off: EnumAutoIndentRule
	Absolute: EnumAutoIndentRule
	Relative: EnumAutoIndentRule
	function GetEnumItems(self): { EnumAutoIndentRule }
	function FromName(self, name: string): EnumAutoIndentRule?
	function FromValue(self, value: number): EnumAutoIndentRule?
end

declare extern type EnumAutomaticSize extends EnumItem with
end

declare extern type EnumAutomaticSize_INTERNAL extends Enum with
	None: EnumAutomaticSize
	X: EnumAutomaticSize
	Y: EnumAutomaticSize
	XY: EnumAutomaticSize
	function GetEnumItems(self): { EnumAutomaticSize }
	function FromName(self, name: string): EnumAutomaticSize?
	function FromValue(self, value: number): EnumAutomaticSize?
end

declare extern type EnumAvatarAssetType extends EnumItem with
end

declare extern type EnumAvatarAssetType_INTERNAL extends Enum with
	TShirt: EnumAvatarAssetType
	Hat: EnumAvatarAssetType
	Shirt: EnumAvatarAssetType
	Pants: EnumAvatarAssetType
	Head: EnumAvatarAssetType
	Face: EnumAvatarAssetType
	Gear: EnumAvatarAssetType
	Torso: EnumAvatarAssetType
	RightArm: EnumAvatarAssetType
	LeftArm: EnumAvatarAssetType
	LeftLeg: EnumAvatarAssetType
	RightLeg: EnumAvatarAssetType
	HairAccessory: EnumAvatarAssetType
	FaceAccessory: EnumAvatarAssetType
	NeckAccessory: EnumAvatarAssetType
	ShoulderAccessory: EnumAvatarAssetType
	FrontAccessory: EnumAvatarAssetType
	BackAccessory: EnumAvatarAssetType
	WaistAccessory: EnumAvatarAssetType
	ClimbAnimation: EnumAvatarAssetType
	FallAnimation: EnumAvatarAssetType
	IdleAnimation: EnumAvatarAssetType
	JumpAnimation: EnumAvatarAssetType
	RunAnimation: EnumAvatarAssetType
	SwimAnimation: EnumAvatarAssetType
	WalkAnimation: EnumAvatarAssetType
	EmoteAnimation: EnumAvatarAssetType
	TShirtAccessory: EnumAvatarAssetType
	ShirtAccessory: EnumAvatarAssetType
	PantsAccessory: EnumAvatarAssetType
	JacketAccessory: EnumAvatarAssetType
	SweaterAccessory: EnumAvatarAssetType
	ShortsAccessory: EnumAvatarAssetType
	LeftShoeAccessory: EnumAvatarAssetType
	RightShoeAccessory: EnumAvatarAssetType
	DressSkirtAccessory: EnumAvatarAssetType
	EyebrowAccessory: EnumAvatarAssetType
	EyelashAccessory: EnumAvatarAssetType
	MoodAnimation: EnumAvatarAssetType
	DynamicHead: EnumAvatarAssetType
	FaceMakeup: EnumAvatarAssetType
	LipMakeup: EnumAvatarAssetType
	EyeMakeup: EnumAvatarAssetType
	AvatarBackground: EnumAvatarAssetType
	function GetEnumItems(self): { EnumAvatarAssetType }
	function FromName(self, name: string): EnumAvatarAssetType?
	function FromValue(self, value: number): EnumAvatarAssetType?
end

declare extern type EnumAvatarChatServiceFeature extends EnumItem with
end

declare extern type EnumAvatarChatServiceFeature_INTERNAL extends Enum with
	None: EnumAvatarChatServiceFeature
	UniverseAudio: EnumAvatarChatServiceFeature
	UniverseVideo: EnumAvatarChatServiceFeature
	PlaceAudio: EnumAvatarChatServiceFeature
	PlaceVideo: EnumAvatarChatServiceFeature
	UserAudioEligible: EnumAvatarChatServiceFeature
	UserAudio: EnumAvatarChatServiceFeature
	UserVideoEligible: EnumAvatarChatServiceFeature
	UserVideo: EnumAvatarChatServiceFeature
	UserBanned: EnumAvatarChatServiceFeature
	UserVerifiedForVoice: EnumAvatarChatServiceFeature
	function GetEnumItems(self): { EnumAvatarChatServiceFeature }
	function FromName(self, name: string): EnumAvatarChatServiceFeature?
	function FromValue(self, value: number): EnumAvatarChatServiceFeature?
end

declare extern type EnumAvatarContextMenuOption extends EnumItem with
end

declare extern type EnumAvatarContextMenuOption_INTERNAL extends Enum with
	Friend: EnumAvatarContextMenuOption
	Chat: EnumAvatarContextMenuOption
	Emote: EnumAvatarContextMenuOption
	InspectMenu: EnumAvatarContextMenuOption
	function GetEnumItems(self): { EnumAvatarContextMenuOption }
	function FromName(self, name: string): EnumAvatarContextMenuOption?
	function FromValue(self, value: number): EnumAvatarContextMenuOption?
end

declare extern type EnumAvatarGenerationError extends EnumItem with
end

declare extern type EnumAvatarGenerationError_INTERNAL extends Enum with
	None: EnumAvatarGenerationError
	Unknown: EnumAvatarGenerationError
	DownloadFailed: EnumAvatarGenerationError
	Canceled: EnumAvatarGenerationError
	Offensive: EnumAvatarGenerationError
	Timeout: EnumAvatarGenerationError
	JobNotFound: EnumAvatarGenerationError
	function GetEnumItems(self): { EnumAvatarGenerationError }
	function FromName(self, name: string): EnumAvatarGenerationError?
	function FromValue(self, value: number): EnumAvatarGenerationError?
end

declare extern type EnumAvatarItemType extends EnumItem with
end

declare extern type EnumAvatarItemType_INTERNAL extends Enum with
	Asset: EnumAvatarItemType
	Bundle: EnumAvatarItemType
	function GetEnumItems(self): { EnumAvatarItemType }
	function FromName(self, name: string): EnumAvatarItemType?
	function FromValue(self, value: number): EnumAvatarItemType?
end

declare extern type EnumAvatarPromptResult extends EnumItem with
end

declare extern type EnumAvatarPromptResult_INTERNAL extends Enum with
	Success: EnumAvatarPromptResult
	PermissionDenied: EnumAvatarPromptResult
	Failed: EnumAvatarPromptResult
	function GetEnumItems(self): { EnumAvatarPromptResult }
	function FromName(self, name: string): EnumAvatarPromptResult?
	function FromValue(self, value: number): EnumAvatarPromptResult?
end

declare extern type EnumAvatarSettingsAccessoryLimitMethod extends EnumItem with
end

declare extern type EnumAvatarSettingsAccessoryLimitMethod_INTERNAL extends Enum with
	Scale: EnumAvatarSettingsAccessoryLimitMethod
	Remove: EnumAvatarSettingsAccessoryLimitMethod
	PreviewScale: EnumAvatarSettingsAccessoryLimitMethod
	PreviewRemove: EnumAvatarSettingsAccessoryLimitMethod
	function GetEnumItems(self): { EnumAvatarSettingsAccessoryLimitMethod }
	function FromName(self, name: string): EnumAvatarSettingsAccessoryLimitMethod?
	function FromValue(self, value: number): EnumAvatarSettingsAccessoryLimitMethod?
end

declare extern type EnumAvatarSettingsAccessoryMode extends EnumItem with
end

declare extern type EnumAvatarSettingsAccessoryMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsAccessoryMode
	CustomLimit: EnumAvatarSettingsAccessoryMode
	function GetEnumItems(self): { EnumAvatarSettingsAccessoryMode }
	function FromName(self, name: string): EnumAvatarSettingsAccessoryMode?
	function FromValue(self, value: number): EnumAvatarSettingsAccessoryMode?
end

declare extern type EnumAvatarSettingsAnimationClipsMode extends EnumItem with
end

declare extern type EnumAvatarSettingsAnimationClipsMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsAnimationClipsMode
	CustomClips: EnumAvatarSettingsAnimationClipsMode
	function GetEnumItems(self): { EnumAvatarSettingsAnimationClipsMode }
	function FromName(self, name: string): EnumAvatarSettingsAnimationClipsMode?
	function FromValue(self, value: number): EnumAvatarSettingsAnimationClipsMode?
end

declare extern type EnumAvatarSettingsAnimationPacksMode extends EnumItem with
end

declare extern type EnumAvatarSettingsAnimationPacksMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsAnimationPacksMode
	StandardR15: EnumAvatarSettingsAnimationPacksMode
	StandardR6: EnumAvatarSettingsAnimationPacksMode
	function GetEnumItems(self): { EnumAvatarSettingsAnimationPacksMode }
	function FromName(self, name: string): EnumAvatarSettingsAnimationPacksMode?
	function FromValue(self, value: number): EnumAvatarSettingsAnimationPacksMode?
end

declare extern type EnumAvatarSettingsAppearanceMode extends EnumItem with
end

declare extern type EnumAvatarSettingsAppearanceMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsAppearanceMode
	CustomParts: EnumAvatarSettingsAppearanceMode
	CustomBody: EnumAvatarSettingsAppearanceMode
	function GetEnumItems(self): { EnumAvatarSettingsAppearanceMode }
	function FromName(self, name: string): EnumAvatarSettingsAppearanceMode?
	function FromValue(self, value: number): EnumAvatarSettingsAppearanceMode?
end

declare extern type EnumAvatarSettingsBuildMode extends EnumItem with
end

declare extern type EnumAvatarSettingsBuildMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsBuildMode
	CustomBuild: EnumAvatarSettingsBuildMode
	function GetEnumItems(self): { EnumAvatarSettingsBuildMode }
	function FromName(self, name: string): EnumAvatarSettingsBuildMode?
	function FromValue(self, value: number): EnumAvatarSettingsBuildMode?
end

declare extern type EnumAvatarSettingsCharacterControllerMode extends EnumItem with
end

declare extern type EnumAvatarSettingsCharacterControllerMode_INTERNAL extends Enum with
	LegacyHumanoid: EnumAvatarSettingsCharacterControllerMode
	LuaCharacterController: EnumAvatarSettingsCharacterControllerMode
	function GetEnumItems(self): { EnumAvatarSettingsCharacterControllerMode }
	function FromName(self, name: string): EnumAvatarSettingsCharacterControllerMode?
	function FromValue(self, value: number): EnumAvatarSettingsCharacterControllerMode?
end

declare extern type EnumAvatarSettingsClothingMode extends EnumItem with
end

declare extern type EnumAvatarSettingsClothingMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsClothingMode
	CustomLimit: EnumAvatarSettingsClothingMode
	function GetEnumItems(self): { EnumAvatarSettingsClothingMode }
	function FromName(self, name: string): EnumAvatarSettingsClothingMode?
	function FromValue(self, value: number): EnumAvatarSettingsClothingMode?
end

declare extern type EnumAvatarSettingsCollisionMode extends EnumItem with
end

declare extern type EnumAvatarSettingsCollisionMode_INTERNAL extends Enum with
	Default: EnumAvatarSettingsCollisionMode
	SingleCollider: EnumAvatarSettingsCollisionMode
	Legacy: EnumAvatarSettingsCollisionMode
	function GetEnumItems(self): { EnumAvatarSettingsCollisionMode }
	function FromName(self, name: string): EnumAvatarSettingsCollisionMode?
	function FromValue(self, value: number): EnumAvatarSettingsCollisionMode?
end

declare extern type EnumAvatarSettingsCustomAccessoryMode extends EnumItem with
end

declare extern type EnumAvatarSettingsCustomAccessoryMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsCustomAccessoryMode
	CustomAccessories: EnumAvatarSettingsCustomAccessoryMode
	function GetEnumItems(self): { EnumAvatarSettingsCustomAccessoryMode }
	function FromName(self, name: string): EnumAvatarSettingsCustomAccessoryMode?
	function FromValue(self, value: number): EnumAvatarSettingsCustomAccessoryMode?
end

declare extern type EnumAvatarSettingsCustomBodyType extends EnumItem with
end

declare extern type EnumAvatarSettingsCustomBodyType_INTERNAL extends Enum with
	AvatarReference: EnumAvatarSettingsCustomBodyType
	BundleId: EnumAvatarSettingsCustomBodyType
	function GetEnumItems(self): { EnumAvatarSettingsCustomBodyType }
	function FromName(self, name: string): EnumAvatarSettingsCustomBodyType?
	function FromValue(self, value: number): EnumAvatarSettingsCustomBodyType?
end

declare extern type EnumAvatarSettingsCustomClothingMode extends EnumItem with
end

declare extern type EnumAvatarSettingsCustomClothingMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsCustomClothingMode
	CustomClothing: EnumAvatarSettingsCustomClothingMode
	function GetEnumItems(self): { EnumAvatarSettingsCustomClothingMode }
	function FromName(self, name: string): EnumAvatarSettingsCustomClothingMode?
	function FromValue(self, value: number): EnumAvatarSettingsCustomClothingMode?
end

declare extern type EnumAvatarSettingsHitAndTouchDetectionMode extends EnumItem with
end

declare extern type EnumAvatarSettingsHitAndTouchDetectionMode_INTERNAL extends Enum with
	UseParts: EnumAvatarSettingsHitAndTouchDetectionMode
	UseCollider: EnumAvatarSettingsHitAndTouchDetectionMode
	function GetEnumItems(self): { EnumAvatarSettingsHitAndTouchDetectionMode }
	function FromName(self, name: string): EnumAvatarSettingsHitAndTouchDetectionMode?
	function FromValue(self, value: number): EnumAvatarSettingsHitAndTouchDetectionMode?
end

declare extern type EnumAvatarSettingsJumpMode extends EnumItem with
end

declare extern type EnumAvatarSettingsJumpMode_INTERNAL extends Enum with
	JumpHeight: EnumAvatarSettingsJumpMode
	JumpPower: EnumAvatarSettingsJumpMode
	function GetEnumItems(self): { EnumAvatarSettingsJumpMode }
	function FromName(self, name: string): EnumAvatarSettingsJumpMode?
	function FromValue(self, value: number): EnumAvatarSettingsJumpMode?
end

declare extern type EnumAvatarSettingsLegacyCollisionMode extends EnumItem with
end

declare extern type EnumAvatarSettingsLegacyCollisionMode_INTERNAL extends Enum with
	R6Colliders: EnumAvatarSettingsLegacyCollisionMode
	InnerBoxColliders: EnumAvatarSettingsLegacyCollisionMode
	function GetEnumItems(self): { EnumAvatarSettingsLegacyCollisionMode }
	function FromName(self, name: string): EnumAvatarSettingsLegacyCollisionMode?
	function FromValue(self, value: number): EnumAvatarSettingsLegacyCollisionMode?
end

declare extern type EnumAvatarSettingsScaleMode extends EnumItem with
end

declare extern type EnumAvatarSettingsScaleMode_INTERNAL extends Enum with
	PlayerChoice: EnumAvatarSettingsScaleMode
	CustomScale: EnumAvatarSettingsScaleMode
	function GetEnumItems(self): { EnumAvatarSettingsScaleMode }
	function FromName(self, name: string): EnumAvatarSettingsScaleMode?
	function FromValue(self, value: number): EnumAvatarSettingsScaleMode?
end

declare extern type EnumAvatarThumbnailCustomizationType extends EnumItem with
end

declare extern type EnumAvatarThumbnailCustomizationType_INTERNAL extends Enum with
	Closeup: EnumAvatarThumbnailCustomizationType
	FullBody: EnumAvatarThumbnailCustomizationType
	function GetEnumItems(self): { EnumAvatarThumbnailCustomizationType }
	function FromName(self, name: string): EnumAvatarThumbnailCustomizationType?
	function FromValue(self, value: number): EnumAvatarThumbnailCustomizationType?
end

declare extern type EnumAvatarUnificationMode extends EnumItem with
end

declare extern type EnumAvatarUnificationMode_INTERNAL extends Enum with
	Default: EnumAvatarUnificationMode
	Disabled: EnumAvatarUnificationMode
	Enabled: EnumAvatarUnificationMode
	function GetEnumItems(self): { EnumAvatarUnificationMode }
	function FromName(self, name: string): EnumAvatarUnificationMode?
	function FromValue(self, value: number): EnumAvatarUnificationMode?
end

declare extern type EnumAxis extends EnumItem with
end

declare extern type EnumAxis_INTERNAL extends Enum with
	X: EnumAxis
	Y: EnumAxis
	Z: EnumAxis
	function GetEnumItems(self): { EnumAxis }
	function FromName(self, name: string): EnumAxis?
	function FromValue(self, value: number): EnumAxis?
end

declare extern type EnumBenefitType extends EnumItem with
end

declare extern type EnumBenefitType_INTERNAL extends Enum with
	DeveloperProduct: EnumBenefitType
	AvatarAsset: EnumBenefitType
	AvatarBundle: EnumBenefitType
	function GetEnumItems(self): { EnumBenefitType }
	function FromName(self, name: string): EnumBenefitType?
	function FromValue(self, value: number): EnumBenefitType?
end

declare extern type EnumBinType extends EnumItem with
end

declare extern type EnumBinType_INTERNAL extends Enum with
	Script: EnumBinType
	GameTool: EnumBinType
	Grab: EnumBinType
	Clone: EnumBinType
	Hammer: EnumBinType
	function GetEnumItems(self): { EnumBinType }
	function FromName(self, name: string): EnumBinType?
	function FromValue(self, value: number): EnumBinType?
end

declare extern type EnumBlendMode extends EnumItem with
end

declare extern type EnumBlendMode_INTERNAL extends Enum with
	Alpha: EnumBlendMode
	Additive: EnumBlendMode
	Multiply: EnumBlendMode
	function GetEnumItems(self): { EnumBlendMode }
	function FromName(self, name: string): EnumBlendMode?
	function FromValue(self, value: number): EnumBlendMode?
end

declare extern type EnumBodyPart extends EnumItem with
end

declare extern type EnumBodyPart_INTERNAL extends Enum with
	Head: EnumBodyPart
	Torso: EnumBodyPart
	LeftArm: EnumBodyPart
	RightArm: EnumBodyPart
	LeftLeg: EnumBodyPart
	RightLeg: EnumBodyPart
	function GetEnumItems(self): { EnumBodyPart }
	function FromName(self, name: string): EnumBodyPart?
	function FromValue(self, value: number): EnumBodyPart?
end

declare extern type EnumBodyPartR15 extends EnumItem with
end

declare extern type EnumBodyPartR15_INTERNAL extends Enum with
	Head: EnumBodyPartR15
	UpperTorso: EnumBodyPartR15
	LowerTorso: EnumBodyPartR15
	LeftFoot: EnumBodyPartR15
	LeftLowerLeg: EnumBodyPartR15
	LeftUpperLeg: EnumBodyPartR15
	RightFoot: EnumBodyPartR15
	RightLowerLeg: EnumBodyPartR15
	RightUpperLeg: EnumBodyPartR15
	LeftHand: EnumBodyPartR15
	LeftLowerArm: EnumBodyPartR15
	LeftUpperArm: EnumBodyPartR15
	RightHand: EnumBodyPartR15
	RightLowerArm: EnumBodyPartR15
	RightUpperArm: EnumBodyPartR15
	RootPart: EnumBodyPartR15
	Unknown: EnumBodyPartR15
	function GetEnumItems(self): { EnumBodyPartR15 }
	function FromName(self, name: string): EnumBodyPartR15?
	function FromValue(self, value: number): EnumBodyPartR15?
end

declare extern type EnumBorderMode extends EnumItem with
end

declare extern type EnumBorderMode_INTERNAL extends Enum with
	Outline: EnumBorderMode
	Middle: EnumBorderMode
	Inset: EnumBorderMode
	function GetEnumItems(self): { EnumBorderMode }
	function FromName(self, name: string): EnumBorderMode?
	function FromValue(self, value: number): EnumBorderMode?
end

declare extern type EnumBorderStrokePosition extends EnumItem with
end

declare extern type EnumBorderStrokePosition_INTERNAL extends Enum with
	Outer: EnumBorderStrokePosition
	Center: EnumBorderStrokePosition
	Inner: EnumBorderStrokePosition
	function GetEnumItems(self): { EnumBorderStrokePosition }
	function FromName(self, name: string): EnumBorderStrokePosition?
	function FromValue(self, value: number): EnumBorderStrokePosition?
end

declare extern type EnumBreakReason extends EnumItem with
end

declare extern type EnumBreakReason_INTERNAL extends Enum with
	Other: EnumBreakReason
	Error: EnumBreakReason
	SpecialBreakpoint: EnumBreakReason
	UserBreakpoint: EnumBreakReason
	function GetEnumItems(self): { EnumBreakReason }
	function FromName(self, name: string): EnumBreakReason?
	function FromValue(self, value: number): EnumBreakReason?
end

declare extern type EnumBreakpointRemoveReason extends EnumItem with
end

declare extern type EnumBreakpointRemoveReason_INTERNAL extends Enum with
	Requested: EnumBreakpointRemoveReason
	ScriptChanged: EnumBreakpointRemoveReason
	ScriptRemoved: EnumBreakpointRemoveReason
	function GetEnumItems(self): { EnumBreakpointRemoveReason }
	function FromName(self, name: string): EnumBreakpointRemoveReason?
	function FromValue(self, value: number): EnumBreakpointRemoveReason?
end

declare extern type EnumBulkMoveMode extends EnumItem with
end

declare extern type EnumBulkMoveMode_INTERNAL extends Enum with
	FireAllEvents: EnumBulkMoveMode
	FireCFrameChanged: EnumBulkMoveMode
	function GetEnumItems(self): { EnumBulkMoveMode }
	function FromName(self, name: string): EnumBulkMoveMode?
	function FromValue(self, value: number): EnumBulkMoveMode?
end

declare extern type EnumBundleType extends EnumItem with
end

declare extern type EnumBundleType_INTERNAL extends Enum with
	BodyParts: EnumBundleType
	Animations: EnumBundleType
	Shoes: EnumBundleType
	DynamicHead: EnumBundleType
	DynamicHeadAvatar: EnumBundleType
	function GetEnumItems(self): { EnumBundleType }
	function FromName(self, name: string): EnumBundleType?
	function FromValue(self, value: number): EnumBundleType?
end

declare extern type EnumButton extends EnumItem with
end

declare extern type EnumButton_INTERNAL extends Enum with
	Dismount: EnumButton
	Jump: EnumButton
	function GetEnumItems(self): { EnumButton }
	function FromName(self, name: string): EnumButton?
	function FromValue(self, value: number): EnumButton?
end

declare extern type EnumButtonStyle extends EnumItem with
end

declare extern type EnumButtonStyle_INTERNAL extends Enum with
	Custom: EnumButtonStyle
	RobloxButtonDefault: EnumButtonStyle
	RobloxButton: EnumButtonStyle
	RobloxRoundButton: EnumButtonStyle
	RobloxRoundDefaultButton: EnumButtonStyle
	RobloxRoundDropdownButton: EnumButtonStyle
	function GetEnumItems(self): { EnumButtonStyle }
	function FromName(self, name: string): EnumButtonStyle?
	function FromValue(self, value: number): EnumButtonStyle?
end

declare extern type EnumCageType extends EnumItem with
end

declare extern type EnumCageType_INTERNAL extends Enum with
	Inner: EnumCageType
	Outer: EnumCageType
	function GetEnumItems(self): { EnumCageType }
	function FromName(self, name: string): EnumCageType?
	function FromValue(self, value: number): EnumCageType?
end

declare extern type EnumCameraMode extends EnumItem with
end

declare extern type EnumCameraMode_INTERNAL extends Enum with
	Classic: EnumCameraMode
	LockFirstPerson: EnumCameraMode
	function GetEnumItems(self): { EnumCameraMode }
	function FromName(self, name: string): EnumCameraMode?
	function FromValue(self, value: number): EnumCameraMode?
end

declare extern type EnumCameraNavigationModel extends EnumItem with
end

declare extern type EnumCameraNavigationModel_INTERNAL extends Enum with
	Roblox: EnumCameraNavigationModel
	IndustryCompatible: EnumCameraNavigationModel
	function GetEnumItems(self): { EnumCameraNavigationModel }
	function FromName(self, name: string): EnumCameraNavigationModel?
	function FromValue(self, value: number): EnumCameraNavigationModel?
end

declare extern type EnumCameraPanMode extends EnumItem with
end

declare extern type EnumCameraPanMode_INTERNAL extends Enum with
	Classic: EnumCameraPanMode
	EdgeBump: EnumCameraPanMode
	function GetEnumItems(self): { EnumCameraPanMode }
	function FromName(self, name: string): EnumCameraPanMode?
	function FromValue(self, value: number): EnumCameraPanMode?
end

declare extern type EnumCameraSpeedAdjustBinding extends EnumItem with
end

declare extern type EnumCameraSpeedAdjustBinding_INTERNAL extends Enum with
	None: EnumCameraSpeedAdjustBinding
	RmbScroll: EnumCameraSpeedAdjustBinding
	AltScroll: EnumCameraSpeedAdjustBinding
	function GetEnumItems(self): { EnumCameraSpeedAdjustBinding }
	function FromName(self, name: string): EnumCameraSpeedAdjustBinding?
	function FromValue(self, value: number): EnumCameraSpeedAdjustBinding?
end

declare extern type EnumCameraType extends EnumItem with
end

declare extern type EnumCameraType_INTERNAL extends Enum with
	Fixed: EnumCameraType
	Attach: EnumCameraType
	Watch: EnumCameraType
	Track: EnumCameraType
	Follow: EnumCameraType
	Custom: EnumCameraType
	Scriptable: EnumCameraType
	Orbital: EnumCameraType
	function GetEnumItems(self): { EnumCameraType }
	function FromName(self, name: string): EnumCameraType?
	function FromValue(self, value: number): EnumCameraType?
end

declare extern type EnumCanCollaborateError extends EnumItem with
end

declare extern type EnumCanCollaborateError_INTERNAL extends Enum with
	Invalid: EnumCanCollaborateError
	None: EnumCanCollaborateError
	NotAgeVerified: EnumCanCollaborateError
	OutsideAgeBucket: EnumCanCollaborateError
	TooManyCollaborators: EnumCanCollaborateError
	PCBlock: EnumCanCollaborateError
	NotFound: EnumCanCollaborateError
	OutsideOwnerAgeBucket: EnumCanCollaborateError
	NotAuthorized: EnumCanCollaborateError
	OutsideAgeBucketTcPc: EnumCanCollaborateError
	OtherCollaboratorSettingsPreventTrust: EnumCanCollaborateError
	AgeVerificationCountryBlocked: EnumCanCollaborateError
	OtherUserCannotCollaborate: EnumCanCollaborateError
	function GetEnumItems(self): { EnumCanCollaborateError }
	function FromName(self, name: string): EnumCanCollaborateError?
	function FromValue(self, value: number): EnumCanCollaborateError?
end

declare extern type EnumCaptureGalleryPermission extends EnumItem with
end

declare extern type EnumCaptureGalleryPermission_INTERNAL extends Enum with
	ReadAndUpload: EnumCaptureGalleryPermission
	function GetEnumItems(self): { EnumCaptureGalleryPermission }
	function FromName(self, name: string): EnumCaptureGalleryPermission?
	function FromValue(self, value: number): EnumCaptureGalleryPermission?
end

declare extern type EnumCaptureType extends EnumItem with
end

declare extern type EnumCaptureType_INTERNAL extends Enum with
	Screenshot: EnumCaptureType
	Video: EnumCaptureType
	function GetEnumItems(self): { EnumCaptureType }
	function FromName(self, name: string): EnumCaptureType?
	function FromValue(self, value: number): EnumCaptureType?
end

declare extern type EnumCatalogCategoryFilter extends EnumItem with
end

declare extern type EnumCatalogCategoryFilter_INTERNAL extends Enum with
	None: EnumCatalogCategoryFilter
	Featured: EnumCatalogCategoryFilter
	Collectibles: EnumCatalogCategoryFilter
	CommunityCreations: EnumCatalogCategoryFilter
	Premium: EnumCatalogCategoryFilter
	Recommended: EnumCatalogCategoryFilter
	function GetEnumItems(self): { EnumCatalogCategoryFilter }
	function FromName(self, name: string): EnumCatalogCategoryFilter?
	function FromValue(self, value: number): EnumCatalogCategoryFilter?
end

declare extern type EnumCatalogSortAggregation extends EnumItem with
end

declare extern type EnumCatalogSortAggregation_INTERNAL extends Enum with
	Past12Hours: EnumCatalogSortAggregation
	PastDay: EnumCatalogSortAggregation
	Past3Days: EnumCatalogSortAggregation
	PastWeek: EnumCatalogSortAggregation
	PastMonth: EnumCatalogSortAggregation
	AllTime: EnumCatalogSortAggregation
	function GetEnumItems(self): { EnumCatalogSortAggregation }
	function FromName(self, name: string): EnumCatalogSortAggregation?
	function FromValue(self, value: number): EnumCatalogSortAggregation?
end

declare extern type EnumCatalogSortType extends EnumItem with
end

declare extern type EnumCatalogSortType_INTERNAL extends Enum with
	Relevance: EnumCatalogSortType
	PriceHighToLow: EnumCatalogSortType
	PriceLowToHigh: EnumCatalogSortType
	MostFavorited: EnumCatalogSortType
	RecentlyCreated: EnumCatalogSortType
	Bestselling: EnumCatalogSortType
	function GetEnumItems(self): { EnumCatalogSortType }
	function FromName(self, name: string): EnumCatalogSortType?
	function FromValue(self, value: number): EnumCatalogSortType?
end

declare extern type EnumCellBlock extends EnumItem with
end

declare extern type EnumCellBlock_INTERNAL extends Enum with
	Solid: EnumCellBlock
	VerticalWedge: EnumCellBlock
	CornerWedge: EnumCellBlock
	InverseCornerWedge: EnumCellBlock
	HorizontalWedge: EnumCellBlock
	function GetEnumItems(self): { EnumCellBlock }
	function FromName(self, name: string): EnumCellBlock?
	function FromValue(self, value: number): EnumCellBlock?
end

declare extern type EnumCellMaterial extends EnumItem with
end

declare extern type EnumCellMaterial_INTERNAL extends Enum with
	Empty: EnumCellMaterial
	Grass: EnumCellMaterial
	Sand: EnumCellMaterial
	Brick: EnumCellMaterial
	Granite: EnumCellMaterial
	Asphalt: EnumCellMaterial
	Iron: EnumCellMaterial
	Aluminum: EnumCellMaterial
	Gold: EnumCellMaterial
	WoodPlank: EnumCellMaterial
	WoodLog: EnumCellMaterial
	Gravel: EnumCellMaterial
	CinderBlock: EnumCellMaterial
	MossyStone: EnumCellMaterial
	Cement: EnumCellMaterial
	RedPlastic: EnumCellMaterial
	BluePlastic: EnumCellMaterial
	Water: EnumCellMaterial
	function GetEnumItems(self): { EnumCellMaterial }
	function FromName(self, name: string): EnumCellMaterial?
	function FromValue(self, value: number): EnumCellMaterial?
end

declare extern type EnumCellOrientation extends EnumItem with
end

declare extern type EnumCellOrientation_INTERNAL extends Enum with
	NegZ: EnumCellOrientation
	X: EnumCellOrientation
	Z: EnumCellOrientation
	NegX: EnumCellOrientation
	function GetEnumItems(self): { EnumCellOrientation }
	function FromName(self, name: string): EnumCellOrientation?
	function FromValue(self, value: number): EnumCellOrientation?
end

declare extern type EnumCenterDialogType extends EnumItem with
end

declare extern type EnumCenterDialogType_INTERNAL extends Enum with
	UnsolicitedDialog: EnumCenterDialogType
	PlayerInitiatedDialog: EnumCenterDialogType
	ModalDialog: EnumCenterDialogType
	QuitDialog: EnumCenterDialogType
	function GetEnumItems(self): { EnumCenterDialogType }
	function FromName(self, name: string): EnumCenterDialogType?
	function FromValue(self, value: number): EnumCenterDialogType?
end

declare extern type EnumCharacterControlMode extends EnumItem with
end

declare extern type EnumCharacterControlMode_INTERNAL extends Enum with
	Default: EnumCharacterControlMode
	Legacy: EnumCharacterControlMode
	NoCharacterController: EnumCharacterControlMode
	LuaCharacterController: EnumCharacterControlMode
	function GetEnumItems(self): { EnumCharacterControlMode }
	function FromName(self, name: string): EnumCharacterControlMode?
	function FromValue(self, value: number): EnumCharacterControlMode?
end

declare extern type EnumChatCallbackType extends EnumItem with
end

declare extern type EnumChatCallbackType_INTERNAL extends Enum with
	OnCreatingChatWindow: EnumChatCallbackType
	OnClientSendingMessage: EnumChatCallbackType
	OnClientFormattingMessage: EnumChatCallbackType
	OnServerReceivingMessage: EnumChatCallbackType
	function GetEnumItems(self): { EnumChatCallbackType }
	function FromName(self, name: string): EnumChatCallbackType?
	function FromValue(self, value: number): EnumChatCallbackType?
end

declare extern type EnumChatColor extends EnumItem with
end

declare extern type EnumChatColor_INTERNAL extends Enum with
	Blue: EnumChatColor
	Green: EnumChatColor
	Red: EnumChatColor
	White: EnumChatColor
	function GetEnumItems(self): { EnumChatColor }
	function FromName(self, name: string): EnumChatColor?
	function FromValue(self, value: number): EnumChatColor?
end

declare extern type EnumChatMode extends EnumItem with
end

declare extern type EnumChatMode_INTERNAL extends Enum with
	Menu: EnumChatMode
	TextAndMenu: EnumChatMode
	function GetEnumItems(self): { EnumChatMode }
	function FromName(self, name: string): EnumChatMode?
	function FromValue(self, value: number): EnumChatMode?
end

declare extern type EnumChatPrivacyMode extends EnumItem with
end

declare extern type EnumChatPrivacyMode_INTERNAL extends Enum with
	AllUsers: EnumChatPrivacyMode
	NoOne: EnumChatPrivacyMode
	Friends: EnumChatPrivacyMode
	function GetEnumItems(self): { EnumChatPrivacyMode }
	function FromName(self, name: string): EnumChatPrivacyMode?
	function FromValue(self, value: number): EnumChatPrivacyMode?
end

declare extern type EnumChatRestrictionStatus extends EnumItem with
end

declare extern type EnumChatRestrictionStatus_INTERNAL extends Enum with
	Unknown: EnumChatRestrictionStatus
	NotRestricted: EnumChatRestrictionStatus
	Restricted: EnumChatRestrictionStatus
	function GetEnumItems(self): { EnumChatRestrictionStatus }
	function FromName(self, name: string): EnumChatRestrictionStatus?
	function FromValue(self, value: number): EnumChatRestrictionStatus?
end

declare extern type EnumChatStyle extends EnumItem with
end

declare extern type EnumChatStyle_INTERNAL extends Enum with
	Classic: EnumChatStyle
	Bubble: EnumChatStyle
	ClassicAndBubble: EnumChatStyle
	function GetEnumItems(self): { EnumChatStyle }
	function FromName(self, name: string): EnumChatStyle?
	function FromValue(self, value: number): EnumChatStyle?
end

declare extern type EnumChatVersion extends EnumItem with
end

declare extern type EnumChatVersion_INTERNAL extends Enum with
	LegacyChatService: EnumChatVersion
	TextChatService: EnumChatVersion
	function GetEnumItems(self): { EnumChatVersion }
	function FromName(self, name: string): EnumChatVersion?
	function FromValue(self, value: number): EnumChatVersion?
end

declare extern type EnumClientAnimatorThrottlingMode extends EnumItem with
end

declare extern type EnumClientAnimatorThrottlingMode_INTERNAL extends Enum with
	Default: EnumClientAnimatorThrottlingMode
	Disabled: EnumClientAnimatorThrottlingMode
	Enabled: EnumClientAnimatorThrottlingMode
	function GetEnumItems(self): { EnumClientAnimatorThrottlingMode }
	function FromName(self, name: string): EnumClientAnimatorThrottlingMode?
	function FromValue(self, value: number): EnumClientAnimatorThrottlingMode?
end

declare extern type EnumCloseReason extends EnumItem with
end

declare extern type EnumCloseReason_INTERNAL extends Enum with
	Unknown: EnumCloseReason
	RobloxMaintenance: EnumCloseReason
	DeveloperShutdown: EnumCloseReason
	DeveloperUpdate: EnumCloseReason
	ServerEmpty: EnumCloseReason
	OutOfMemory: EnumCloseReason
	Moderation: EnumCloseReason
	function GetEnumItems(self): { EnumCloseReason }
	function FromName(self, name: string): EnumCloseReason?
	function FromValue(self, value: number): EnumCloseReason?
end

declare extern type EnumCollaboratorStatus extends EnumItem with
end

declare extern type EnumCollaboratorStatus_INTERNAL extends Enum with
	None: EnumCollaboratorStatus
	Editing3D: EnumCollaboratorStatus
	Scripting: EnumCollaboratorStatus
	PrivateScripting: EnumCollaboratorStatus
	function GetEnumItems(self): { EnumCollaboratorStatus }
	function FromName(self, name: string): EnumCollaboratorStatus?
	function FromValue(self, value: number): EnumCollaboratorStatus?
end

declare extern type EnumCollisionFidelity extends EnumItem with
end

declare extern type EnumCollisionFidelity_INTERNAL extends Enum with
	Default: EnumCollisionFidelity
	Hull: EnumCollisionFidelity
	Box: EnumCollisionFidelity
	PreciseConvexDecomposition: EnumCollisionFidelity
	Scalable: EnumCollisionFidelity
	function GetEnumItems(self): { EnumCollisionFidelity }
	function FromName(self, name: string): EnumCollisionFidelity?
	function FromValue(self, value: number): EnumCollisionFidelity?
end

declare extern type EnumCommandPermission extends EnumItem with
end

declare extern type EnumCommandPermission_INTERNAL extends Enum with
	Plugin: EnumCommandPermission
	LocalUser: EnumCommandPermission
	function GetEnumItems(self): { EnumCommandPermission }
	function FromName(self, name: string): EnumCommandPermission?
	function FromValue(self, value: number): EnumCommandPermission?
end

declare extern type EnumCompileTarget extends EnumItem with
end

declare extern type EnumCompileTarget_INTERNAL extends Enum with
	Client: EnumCompileTarget
	CoreScript: EnumCompileTarget
	Studio: EnumCompileTarget
	CoreScriptRaw: EnumCompileTarget
	function GetEnumItems(self): { EnumCompileTarget }
	function FromName(self, name: string): EnumCompileTarget?
	function FromValue(self, value: number): EnumCompileTarget?
end

declare extern type EnumCompletionAcceptanceBehavior extends EnumItem with
end

declare extern type EnumCompletionAcceptanceBehavior_INTERNAL extends Enum with
	Insert: EnumCompletionAcceptanceBehavior
	Replace: EnumCompletionAcceptanceBehavior
	ReplaceOnEnterInsertOnTab: EnumCompletionAcceptanceBehavior
	InsertOnEnterReplaceOnTab: EnumCompletionAcceptanceBehavior
	function GetEnumItems(self): { EnumCompletionAcceptanceBehavior }
	function FromName(self, name: string): EnumCompletionAcceptanceBehavior?
	function FromValue(self, value: number): EnumCompletionAcceptanceBehavior?
end

declare extern type EnumCompletionItemKind extends EnumItem with
end

declare extern type EnumCompletionItemKind_INTERNAL extends Enum with
	Text: EnumCompletionItemKind
	Method: EnumCompletionItemKind
	Function: EnumCompletionItemKind
	Constructor: EnumCompletionItemKind
	Field: EnumCompletionItemKind
	Variable: EnumCompletionItemKind
	Class: EnumCompletionItemKind
	Interface: EnumCompletionItemKind
	Module: EnumCompletionItemKind
	Property: EnumCompletionItemKind
	Unit: EnumCompletionItemKind
	Value: EnumCompletionItemKind
	Enum: EnumCompletionItemKind
	Keyword: EnumCompletionItemKind
	Snippet: EnumCompletionItemKind
	Color: EnumCompletionItemKind
	File: EnumCompletionItemKind
	Reference: EnumCompletionItemKind
	Folder: EnumCompletionItemKind
	EnumMember: EnumCompletionItemKind
	Constant: EnumCompletionItemKind
	Struct: EnumCompletionItemKind
	Event: EnumCompletionItemKind
	Operator: EnumCompletionItemKind
	TypeParameter: EnumCompletionItemKind
	function GetEnumItems(self): { EnumCompletionItemKind }
	function FromName(self, name: string): EnumCompletionItemKind?
	function FromValue(self, value: number): EnumCompletionItemKind?
end

declare extern type EnumCompletionItemTag extends EnumItem with
end

declare extern type EnumCompletionItemTag_INTERNAL extends Enum with
	Deprecated: EnumCompletionItemTag
	IncorrectIndexType: EnumCompletionItemTag
	PluginPermissions: EnumCompletionItemTag
	CommandLinePermissions: EnumCompletionItemTag
	RobloxPermissions: EnumCompletionItemTag
	AddParens: EnumCompletionItemTag
	PutCursorInParens: EnumCompletionItemTag
	TypeCorrect: EnumCompletionItemTag
	ClientServerBoundaryViolation: EnumCompletionItemTag
	Invalidated: EnumCompletionItemTag
	PutCursorBeforeEnd: EnumCompletionItemTag
	function GetEnumItems(self): { EnumCompletionItemTag }
	function FromName(self, name: string): EnumCompletionItemTag?
	function FromValue(self, value: number): EnumCompletionItemTag?
end

declare extern type EnumCompletionTriggerKind extends EnumItem with
end

declare extern type EnumCompletionTriggerKind_INTERNAL extends Enum with
	Invoked: EnumCompletionTriggerKind
	TriggerCharacter: EnumCompletionTriggerKind
	TriggerForIncompleteCompletions: EnumCompletionTriggerKind
	function GetEnumItems(self): { EnumCompletionTriggerKind }
	function FromName(self, name: string): EnumCompletionTriggerKind?
	function FromValue(self, value: number): EnumCompletionTriggerKind?
end

declare extern type EnumCompositeValueCurveType extends EnumItem with
end

declare extern type EnumCompositeValueCurveType_INTERNAL extends Enum with
	ColorRGB: EnumCompositeValueCurveType
	ColorHSV: EnumCompositeValueCurveType
	NumberRange: EnumCompositeValueCurveType
	Rect: EnumCompositeValueCurveType
	UDim: EnumCompositeValueCurveType
	UDim2: EnumCompositeValueCurveType
	Vector2: EnumCompositeValueCurveType
	Vector3: EnumCompositeValueCurveType
	function GetEnumItems(self): { EnumCompositeValueCurveType }
	function FromName(self, name: string): EnumCompositeValueCurveType?
	function FromValue(self, value: number): EnumCompositeValueCurveType?
end

declare extern type EnumCompressionAlgorithm extends EnumItem with
end

declare extern type EnumCompressionAlgorithm_INTERNAL extends Enum with
	Zstd: EnumCompressionAlgorithm
	function GetEnumItems(self): { EnumCompressionAlgorithm }
	function FromName(self, name: string): EnumCompressionAlgorithm?
	function FromValue(self, value: number): EnumCompressionAlgorithm?
end

declare extern type EnumComputerCameraMovementMode extends EnumItem with
end

declare extern type EnumComputerCameraMovementMode_INTERNAL extends Enum with
	Default: EnumComputerCameraMovementMode
	Classic: EnumComputerCameraMovementMode
	Follow: EnumComputerCameraMovementMode
	Orbital: EnumComputerCameraMovementMode
	CameraToggle: EnumComputerCameraMovementMode
	function GetEnumItems(self): { EnumComputerCameraMovementMode }
	function FromName(self, name: string): EnumComputerCameraMovementMode?
	function FromValue(self, value: number): EnumComputerCameraMovementMode?
end

declare extern type EnumComputerMovementMode extends EnumItem with
end

declare extern type EnumComputerMovementMode_INTERNAL extends Enum with
	Default: EnumComputerMovementMode
	KeyboardMouse: EnumComputerMovementMode
	ClickToMove: EnumComputerMovementMode
	function GetEnumItems(self): { EnumComputerMovementMode }
	function FromName(self, name: string): EnumComputerMovementMode?
	function FromValue(self, value: number): EnumComputerMovementMode?
end

declare extern type EnumConfigSnapshotErrorState extends EnumItem with
end

declare extern type EnumConfigSnapshotErrorState_INTERNAL extends Enum with
	None: EnumConfigSnapshotErrorState
	LoadFailed: EnumConfigSnapshotErrorState
	function GetEnumItems(self): { EnumConfigSnapshotErrorState }
	function FromName(self, name: string): EnumConfigSnapshotErrorState?
	function FromValue(self, value: number): EnumConfigSnapshotErrorState?
end

declare extern type EnumConnectionError extends EnumItem with
end

declare extern type EnumConnectionError_INTERNAL extends Enum with
	OK: EnumConnectionError
	Unknown: EnumConnectionError
	ConnectErrors: EnumConnectionError
	AlreadyConnected: EnumConnectionError
	NoFreeIncomingConnections: EnumConnectionError
	ConnectionBanned: EnumConnectionError
	InvalidPassword: EnumConnectionError
	IncompatibleProtocolVersion: EnumConnectionError
	IPRecentlyConnected: EnumConnectionError
	OurSystemRequiresSecurity: EnumConnectionError
	SecurityKeyMismatch: EnumConnectionError
	DisconnectErrors: EnumConnectionError
	DisconnectBadhash: EnumConnectionError
	DisconnectSecurityKeyMismatch: EnumConnectionError
	DisconnectProtocolMismatch: EnumConnectionError
	DisconnectReceivePacketError: EnumConnectionError
	DisconnectReceivePacketStreamError: EnumConnectionError
	DisconnectSendPacketError: EnumConnectionError
	DisconnectIllegalTeleport: EnumConnectionError
	DisconnectDuplicatePlayer: EnumConnectionError
	DisconnectDuplicateTicket: EnumConnectionError
	DisconnectTimeout: EnumConnectionError
	DisconnectLuaKick: EnumConnectionError
	DisconnectOnRemoteSysStats: EnumConnectionError
	DisconnectHashTimeout: EnumConnectionError
	DisconnectCloudEditKick: EnumConnectionError
	DisconnectPlayerless: EnumConnectionError
	DisconnectNewSecurityKeyMismatch: EnumConnectionError
	DisconnectEvicted: EnumConnectionError
	DisconnectDevMaintenance: EnumConnectionError
	DisconnectRobloxMaintenance: EnumConnectionError
	DisconnectRejoin: EnumConnectionError
	DisconnectConnectionLost: EnumConnectionError
	DisconnectIdle: EnumConnectionError
	DisconnectRaknetErrors: EnumConnectionError
	DisconnectWrongVersion: EnumConnectionError
	DisconnectBySecurityPolicy: EnumConnectionError
	DisconnectBlockedIP: EnumConnectionError
	DisconnectClientFailure: EnumConnectionError
	DisconnectClientRequest: EnumConnectionError
	DisconnectPrivateServerKickout: EnumConnectionError
	DisconnectModeratedGame: EnumConnectionError
	ServerShutdown: EnumConnectionError
	ReplicatorTimeout: EnumConnectionError
	PlayerRemoved: EnumConnectionError
	DisconnectOutOfMemoryKeepPlayingLeave: EnumConnectionError
	DisconnectRomarkEndOfTest: EnumConnectionError
	DisconnectCollaboratorPermissionRevoked: EnumConnectionError
	DisconnectCollaboratorUnderage: EnumConnectionError
	NetworkInternal: EnumConnectionError
	NetworkSend: EnumConnectionError
	NetworkTimeout: EnumConnectionError
	NetworkMisbehavior: EnumConnectionError
	NetworkSecurity: EnumConnectionError
	ReplacementReady: EnumConnectionError
	ServerEmpty: EnumConnectionError
	PhantomFreeze: EnumConnectionError
	AndroidAnticheatKick: EnumConnectionError
	AndroidEmulatorKick: EnumConnectionError
	AndroidRootedKick: EnumConnectionError
	ScreentimeLockoutKick: EnumConnectionError
	DisconnectionNotification: EnumConnectionError
	DisconnectVerboselyModeratedGame: EnumConnectionError
	DisconnectCollaboratorNotAgeVerified: EnumConnectionError
	DisconnectCollaboratorTrustedConnectionsRequired: EnumConnectionError
	DisconnectCollaboratorOwnerActionRequired: EnumConnectionError
	DisconnectCollaboratorTooManyCollaborators: EnumConnectionError
	DisconnectCollaboratorUnknownError: EnumConnectionError
	DisconnectCollaboratorRequestedEviction: EnumConnectionError
	DisconnectCollaboratorTrustedConnectionsRequiredPC: EnumConnectionError
	DisconnectRemoteAttestationUnsupported: EnumConnectionError
	DisconnectRemoteAttestationGeneralFailure: EnumConnectionError
	DisconnectRemoteAttestationTimeout: EnumConnectionError
	DisconnectRemoteAttestationOSOutOfDate: EnumConnectionError
	DisconnectRemoteAttestationBootValidationFailure: EnumConnectionError
	PlacelaunchErrors: EnumConnectionError
	PlacelaunchDisabled: EnumConnectionError
	PlacelaunchError: EnumConnectionError
	PlacelaunchGameEnded: EnumConnectionError
	PlacelaunchGameFull: EnumConnectionError
	PlacelaunchUserLeft: EnumConnectionError
	PlacelaunchRestricted: EnumConnectionError
	PlacelaunchUnauthorized: EnumConnectionError
	PlacelaunchFlooded: EnumConnectionError
	PlacelaunchHashExpired: EnumConnectionError
	PlacelaunchHashException: EnumConnectionError
	PlacelaunchPartyCannotFit: EnumConnectionError
	PlacelaunchHttpError: EnumConnectionError
	PlacelaunchUserPrivacyUnauthorized: EnumConnectionError
	PlacelaunchVipOwnerNotPresent: EnumConnectionError
	PlacelaunchAgeVerificationRequired: EnumConnectionError
	PlacelaunchParentalApprovalRequired: EnumConnectionError
	PlacelaunchCoreGated: EnumConnectionError
	PlacelaunchCollaborationCoreGated: EnumConnectionError
	PlacelaunchCreatorBan: EnumConnectionError
	PlacelaunchDeviceBlock: EnumConnectionError
	PlacelaunchCustomMessage: EnumConnectionError
	PlacelaunchOtherError: EnumConnectionError
	TeleportErrors: EnumConnectionError
	TeleportFailure: EnumConnectionError
	TeleportGameNotFound: EnumConnectionError
	TeleportGameEnded: EnumConnectionError
	TeleportGameFull: EnumConnectionError
	TeleportUnauthorized: EnumConnectionError
	TeleportFlooded: EnumConnectionError
	TeleportIsTeleporting: EnumConnectionError
	function GetEnumItems(self): { EnumConnectionError }
	function FromName(self, name: string): EnumConnectionError?
	function FromValue(self, value: number): EnumConnectionError?
end

declare extern type EnumConnectionState extends EnumItem with
end

declare extern type EnumConnectionState_INTERNAL extends Enum with
	Connected: EnumConnectionState
	Disconnected: EnumConnectionState
	function GetEnumItems(self): { EnumConnectionState }
	function FromName(self, name: string): EnumConnectionState?
	function FromValue(self, value: number): EnumConnectionState?
end

declare extern type EnumContentSourceType extends EnumItem with
end

declare extern type EnumContentSourceType_INTERNAL extends Enum with
	None: EnumContentSourceType
	Uri: EnumContentSourceType
	Object: EnumContentSourceType
	Opaque: EnumContentSourceType
	function GetEnumItems(self): { EnumContentSourceType }
	function FromName(self, name: string): EnumContentSourceType?
	function FromValue(self, value: number): EnumContentSourceType?
end

declare extern type EnumContextActionPriority extends EnumItem with
end

declare extern type EnumContextActionPriority_INTERNAL extends Enum with
	Low: EnumContextActionPriority
	Medium: EnumContextActionPriority
	High: EnumContextActionPriority
	function GetEnumItems(self): { EnumContextActionPriority }
	function FromName(self, name: string): EnumContextActionPriority?
	function FromValue(self, value: number): EnumContextActionPriority?
end

declare extern type EnumContextActionResult extends EnumItem with
end

declare extern type EnumContextActionResult_INTERNAL extends Enum with
	Sink: EnumContextActionResult
	Pass: EnumContextActionResult
	function GetEnumItems(self): { EnumContextActionResult }
	function FromName(self, name: string): EnumContextActionResult?
	function FromValue(self, value: number): EnumContextActionResult?
end

declare extern type EnumControlMode extends EnumItem with
end

declare extern type EnumControlMode_INTERNAL extends Enum with
	Classic: EnumControlMode
	MouseLockSwitch: EnumControlMode
	function GetEnumItems(self): { EnumControlMode }
	function FromName(self, name: string): EnumControlMode?
	function FromValue(self, value: number): EnumControlMode?
end

declare extern type EnumCoreGuiType extends EnumItem with
end

declare extern type EnumCoreGuiType_INTERNAL extends Enum with
	PlayerList: EnumCoreGuiType
	Health: EnumCoreGuiType
	Backpack: EnumCoreGuiType
	Chat: EnumCoreGuiType
	All: EnumCoreGuiType
	EmotesMenu: EnumCoreGuiType
	SelfView: EnumCoreGuiType
	Captures: EnumCoreGuiType
	AvatarSwitcher: EnumCoreGuiType
	ExperienceShop: EnumCoreGuiType
	function GetEnumItems(self): { EnumCoreGuiType }
	function FromName(self, name: string): EnumCoreGuiType?
	function FromValue(self, value: number): EnumCoreGuiType?
end

declare extern type EnumCreateAssetResult extends EnumItem with
end

declare extern type EnumCreateAssetResult_INTERNAL extends Enum with
	Success: EnumCreateAssetResult
	PermissionDenied: EnumCreateAssetResult
	UploadFailed: EnumCreateAssetResult
	Unknown: EnumCreateAssetResult
	function GetEnumItems(self): { EnumCreateAssetResult }
	function FromName(self, name: string): EnumCreateAssetResult?
	function FromValue(self, value: number): EnumCreateAssetResult?
end

declare extern type EnumCreateContentResult extends EnumItem with
end

declare extern type EnumCreateContentResult_INTERNAL extends Enum with
	Success: EnumCreateContentResult
	PermissionDenied: EnumCreateContentResult
	UploadFailed: EnumCreateContentResult
	StorageLimitExceeded: EnumCreateContentResult
	Unknown: EnumCreateContentResult
	function GetEnumItems(self): { EnumCreateContentResult }
	function FromName(self, name: string): EnumCreateContentResult?
	function FromValue(self, value: number): EnumCreateContentResult?
end

declare extern type EnumCreateOutfitFailure extends EnumItem with
end

declare extern type EnumCreateOutfitFailure_INTERNAL extends Enum with
	InvalidName: EnumCreateOutfitFailure
	OutfitLimitReached: EnumCreateOutfitFailure
	Other: EnumCreateOutfitFailure
	function GetEnumItems(self): { EnumCreateOutfitFailure }
	function FromName(self, name: string): EnumCreateOutfitFailure?
	function FromValue(self, value: number): EnumCreateOutfitFailure?
end

declare extern type EnumCreatorType extends EnumItem with
end

declare extern type EnumCreatorType_INTERNAL extends Enum with
	User: EnumCreatorType
	Group: EnumCreatorType
	function GetEnumItems(self): { EnumCreatorType }
	function FromName(self, name: string): EnumCreatorType?
	function FromValue(self, value: number): EnumCreatorType?
end

declare extern type EnumCreatorTypeFilter extends EnumItem with
end

declare extern type EnumCreatorTypeFilter_INTERNAL extends Enum with
	User: EnumCreatorTypeFilter
	Group: EnumCreatorTypeFilter
	All: EnumCreatorTypeFilter
	function GetEnumItems(self): { EnumCreatorTypeFilter }
	function FromName(self, name: string): EnumCreatorTypeFilter?
	function FromValue(self, value: number): EnumCreatorTypeFilter?
end

declare extern type EnumCurrencyType extends EnumItem with
end

declare extern type EnumCurrencyType_INTERNAL extends Enum with
	Default: EnumCurrencyType
	Robux: EnumCurrencyType
	Tix: EnumCurrencyType
	function GetEnumItems(self): { EnumCurrencyType }
	function FromName(self, name: string): EnumCurrencyType?
	function FromValue(self, value: number): EnumCurrencyType?
end

declare extern type EnumCustomCameraMode extends EnumItem with
end

declare extern type EnumCustomCameraMode_INTERNAL extends Enum with
	Default: EnumCustomCameraMode
	Classic: EnumCustomCameraMode
	Follow: EnumCustomCameraMode
	function GetEnumItems(self): { EnumCustomCameraMode }
	function FromName(self, name: string): EnumCustomCameraMode?
	function FromValue(self, value: number): EnumCustomCameraMode?
end

declare extern type EnumDataModelExtractorFileType extends EnumItem with
end

declare extern type EnumDataModelExtractorFileType_INTERNAL extends Enum with
	PlaceFile: EnumDataModelExtractorFileType
	FirstSlice: EnumDataModelExtractorFileType
	NonFirstSlice: EnumDataModelExtractorFileType
	function GetEnumItems(self): { EnumDataModelExtractorFileType }
	function FromName(self, name: string): EnumDataModelExtractorFileType?
	function FromValue(self, value: number): EnumDataModelExtractorFileType?
end

declare extern type EnumDataStoreRequestType extends EnumItem with
end

declare extern type EnumDataStoreRequestType_INTERNAL extends Enum with
	GetAsync: EnumDataStoreRequestType
	SetIncrementAsync: EnumDataStoreRequestType
	UpdateAsync: EnumDataStoreRequestType
	GetSortedAsync: EnumDataStoreRequestType
	SetIncrementSortedAsync: EnumDataStoreRequestType
	OnUpdate: EnumDataStoreRequestType
	ListAsync: EnumDataStoreRequestType
	GetVersionAsync: EnumDataStoreRequestType
	RemoveVersionAsync: EnumDataStoreRequestType
	StandardRead: EnumDataStoreRequestType
	StandardWrite: EnumDataStoreRequestType
	StandardList: EnumDataStoreRequestType
	StandardRemove: EnumDataStoreRequestType
	OrderedRead: EnumDataStoreRequestType
	OrderedWrite: EnumDataStoreRequestType
	OrderedList: EnumDataStoreRequestType
	OrderedRemove: EnumDataStoreRequestType
	function GetEnumItems(self): { EnumDataStoreRequestType }
	function FromName(self, name: string): EnumDataStoreRequestType?
	function FromValue(self, value: number): EnumDataStoreRequestType?
end

declare extern type EnumDebugBreakModeType extends EnumItem with
end

declare extern type EnumDebugBreakModeType_INTERNAL extends Enum with
	Never: EnumDebugBreakModeType
	Always: EnumDebugBreakModeType
	Unhandled: EnumDebugBreakModeType
	function GetEnumItems(self): { EnumDebugBreakModeType }
	function FromName(self, name: string): EnumDebugBreakModeType?
	function FromValue(self, value: number): EnumDebugBreakModeType?
end

declare extern type EnumDebuggerEndReason extends EnumItem with
end

declare extern type EnumDebuggerEndReason_INTERNAL extends Enum with
	ClientRequest: EnumDebuggerEndReason
	Timeout: EnumDebuggerEndReason
	InvalidHost: EnumDebuggerEndReason
	Disconnected: EnumDebuggerEndReason
	ServerShutdown: EnumDebuggerEndReason
	ServerProtocolMismatch: EnumDebuggerEndReason
	ConfigurationFailed: EnumDebuggerEndReason
	RpcError: EnumDebuggerEndReason
	function GetEnumItems(self): { EnumDebuggerEndReason }
	function FromName(self, name: string): EnumDebuggerEndReason?
	function FromValue(self, value: number): EnumDebuggerEndReason?
end

declare extern type EnumDebuggerExceptionBreakMode extends EnumItem with
end

declare extern type EnumDebuggerExceptionBreakMode_INTERNAL extends Enum with
	Never: EnumDebuggerExceptionBreakMode
	Always: EnumDebuggerExceptionBreakMode
	Unhandled: EnumDebuggerExceptionBreakMode
	function GetEnumItems(self): { EnumDebuggerExceptionBreakMode }
	function FromName(self, name: string): EnumDebuggerExceptionBreakMode?
	function FromValue(self, value: number): EnumDebuggerExceptionBreakMode?
end

declare extern type EnumDebuggerFrameType extends EnumItem with
end

declare extern type EnumDebuggerFrameType_INTERNAL extends Enum with
	C: EnumDebuggerFrameType
	Lua: EnumDebuggerFrameType
	function GetEnumItems(self): { EnumDebuggerFrameType }
	function FromName(self, name: string): EnumDebuggerFrameType?
	function FromValue(self, value: number): EnumDebuggerFrameType?
end

declare extern type EnumDebuggerPauseReason extends EnumItem with
end

declare extern type EnumDebuggerPauseReason_INTERNAL extends Enum with
	Unknown: EnumDebuggerPauseReason
	Requested: EnumDebuggerPauseReason
	Breakpoint: EnumDebuggerPauseReason
	Exception: EnumDebuggerPauseReason
	SingleStep: EnumDebuggerPauseReason
	Entrypoint: EnumDebuggerPauseReason
	function GetEnumItems(self): { EnumDebuggerPauseReason }
	function FromName(self, name: string): EnumDebuggerPauseReason?
	function FromValue(self, value: number): EnumDebuggerPauseReason?
end

declare extern type EnumDebuggerResumeType extends EnumItem with
end

declare extern type EnumDebuggerResumeType_INTERNAL extends Enum with
	StepInto: EnumDebuggerResumeType
	StepOut: EnumDebuggerResumeType
	StepOver: EnumDebuggerResumeType
	Resume: EnumDebuggerResumeType
	function GetEnumItems(self): { EnumDebuggerResumeType }
	function FromName(self, name: string): EnumDebuggerResumeType?
	function FromValue(self, value: number): EnumDebuggerResumeType?
end

declare extern type EnumDebuggerStatus extends EnumItem with
end

declare extern type EnumDebuggerStatus_INTERNAL extends Enum with
	Success: EnumDebuggerStatus
	Timeout: EnumDebuggerStatus
	ConnectionLost: EnumDebuggerStatus
	InvalidResponse: EnumDebuggerStatus
	InternalError: EnumDebuggerStatus
	InvalidState: EnumDebuggerStatus
	RpcError: EnumDebuggerStatus
	InvalidArgument: EnumDebuggerStatus
	ConnectionClosed: EnumDebuggerStatus
	function GetEnumItems(self): { EnumDebuggerStatus }
	function FromName(self, name: string): EnumDebuggerStatus?
	function FromValue(self, value: number): EnumDebuggerStatus?
end

declare extern type EnumDefaultScriptSyncFileType extends EnumItem with
end

declare extern type EnumDefaultScriptSyncFileType_INTERNAL extends Enum with
	Lua: EnumDefaultScriptSyncFileType
	Luau: EnumDefaultScriptSyncFileType
	function GetEnumItems(self): { EnumDefaultScriptSyncFileType }
	function FromName(self, name: string): EnumDefaultScriptSyncFileType?
	function FromValue(self, value: number): EnumDefaultScriptSyncFileType?
end

declare extern type EnumDevCameraOcclusionMode extends EnumItem with
end

declare extern type EnumDevCameraOcclusionMode_INTERNAL extends Enum with
	Zoom: EnumDevCameraOcclusionMode
	Invisicam: EnumDevCameraOcclusionMode
	function GetEnumItems(self): { EnumDevCameraOcclusionMode }
	function FromName(self, name: string): EnumDevCameraOcclusionMode?
	function FromValue(self, value: number): EnumDevCameraOcclusionMode?
end

declare extern type EnumDevComputerCameraMovementMode extends EnumItem with
end

declare extern type EnumDevComputerCameraMovementMode_INTERNAL extends Enum with
	UserChoice: EnumDevComputerCameraMovementMode
	Classic: EnumDevComputerCameraMovementMode
	Follow: EnumDevComputerCameraMovementMode
	Orbital: EnumDevComputerCameraMovementMode
	CameraToggle: EnumDevComputerCameraMovementMode
	function GetEnumItems(self): { EnumDevComputerCameraMovementMode }
	function FromName(self, name: string): EnumDevComputerCameraMovementMode?
	function FromValue(self, value: number): EnumDevComputerCameraMovementMode?
end

declare extern type EnumDevComputerMovementMode extends EnumItem with
end

declare extern type EnumDevComputerMovementMode_INTERNAL extends Enum with
	UserChoice: EnumDevComputerMovementMode
	KeyboardMouse: EnumDevComputerMovementMode
	ClickToMove: EnumDevComputerMovementMode
	Scriptable: EnumDevComputerMovementMode
	function GetEnumItems(self): { EnumDevComputerMovementMode }
	function FromName(self, name: string): EnumDevComputerMovementMode?
	function FromValue(self, value: number): EnumDevComputerMovementMode?
end

declare extern type EnumDevTouchCameraMovementMode extends EnumItem with
end

declare extern type EnumDevTouchCameraMovementMode_INTERNAL extends Enum with
	UserChoice: EnumDevTouchCameraMovementMode
	Classic: EnumDevTouchCameraMovementMode
	Follow: EnumDevTouchCameraMovementMode
	Orbital: EnumDevTouchCameraMovementMode
	function GetEnumItems(self): { EnumDevTouchCameraMovementMode }
	function FromName(self, name: string): EnumDevTouchCameraMovementMode?
	function FromValue(self, value: number): EnumDevTouchCameraMovementMode?
end

declare extern type EnumDevTouchMovementMode extends EnumItem with
end

declare extern type EnumDevTouchMovementMode_INTERNAL extends Enum with
	UserChoice: EnumDevTouchMovementMode
	Thumbstick: EnumDevTouchMovementMode
	DPad: EnumDevTouchMovementMode
	Thumbpad: EnumDevTouchMovementMode
	ClickToMove: EnumDevTouchMovementMode
	Scriptable: EnumDevTouchMovementMode
	DynamicThumbstick: EnumDevTouchMovementMode
	function GetEnumItems(self): { EnumDevTouchMovementMode }
	function FromName(self, name: string): EnumDevTouchMovementMode?
	function FromValue(self, value: number): EnumDevTouchMovementMode?
end

declare extern type EnumDeveloperMemoryTag extends EnumItem with
end

declare extern type EnumDeveloperMemoryTag_INTERNAL extends Enum with
	Internal: EnumDeveloperMemoryTag
	HttpCache: EnumDeveloperMemoryTag
	Instances: EnumDeveloperMemoryTag
	Signals: EnumDeveloperMemoryTag
	LuaHeap: EnumDeveloperMemoryTag
	Script: EnumDeveloperMemoryTag
	PhysicsCollision: EnumDeveloperMemoryTag
	BaseParts: EnumDeveloperMemoryTag
	GraphicsSolidModels: EnumDeveloperMemoryTag
	GraphicsMeshParts: EnumDeveloperMemoryTag
	GraphicsParticles: EnumDeveloperMemoryTag
	GraphicsParts: EnumDeveloperMemoryTag
	GraphicsSpatialHash: EnumDeveloperMemoryTag
	GraphicsTerrain: EnumDeveloperMemoryTag
	GraphicsTexture: EnumDeveloperMemoryTag
	GraphicsTextureCharacter: EnumDeveloperMemoryTag
	Sounds: EnumDeveloperMemoryTag
	StreamingSounds: EnumDeveloperMemoryTag
	TerrainVoxels: EnumDeveloperMemoryTag
	Gui: EnumDeveloperMemoryTag
	Animation: EnumDeveloperMemoryTag
	Navigation: EnumDeveloperMemoryTag
	GeometryCSG: EnumDeveloperMemoryTag
	GraphicsSlimModels: EnumDeveloperMemoryTag
	function GetEnumItems(self): { EnumDeveloperMemoryTag }
	function FromName(self, name: string): EnumDeveloperMemoryTag?
	function FromValue(self, value: number): EnumDeveloperMemoryTag?
end

declare extern type EnumDeviceFeatureType extends EnumItem with
end

declare extern type EnumDeviceFeatureType_INTERNAL extends Enum with
	DeviceCapture: EnumDeviceFeatureType
	InExperienceFAE: EnumDeviceFeatureType
	function GetEnumItems(self): { EnumDeviceFeatureType }
	function FromName(self, name: string): EnumDeviceFeatureType?
	function FromValue(self, value: number): EnumDeviceFeatureType?
end

declare extern type EnumDeviceForm extends EnumItem with
end

declare extern type EnumDeviceForm_INTERNAL extends Enum with
	Console: EnumDeviceForm
	Phone: EnumDeviceForm
	Tablet: EnumDeviceForm
	Desktop: EnumDeviceForm
	VR: EnumDeviceForm
	function GetEnumItems(self): { EnumDeviceForm }
	function FromName(self, name: string): EnumDeviceForm?
	function FromValue(self, value: number): EnumDeviceForm?
end

declare extern type EnumDeviceLevel extends EnumItem with
end

declare extern type EnumDeviceLevel_INTERNAL extends Enum with
	Low: EnumDeviceLevel
	Medium: EnumDeviceLevel
	High: EnumDeviceLevel
	function GetEnumItems(self): { EnumDeviceLevel }
	function FromName(self, name: string): EnumDeviceLevel?
	function FromValue(self, value: number): EnumDeviceLevel?
end

declare extern type EnumDeviceSimulatorScalingMode extends EnumItem with
end

declare extern type EnumDeviceSimulatorScalingMode_INTERNAL extends Enum with
	ScaleToPhysicalSize: EnumDeviceSimulatorScalingMode
	ActualResolution: EnumDeviceSimulatorScalingMode
	FitToWindow: EnumDeviceSimulatorScalingMode
	function GetEnumItems(self): { EnumDeviceSimulatorScalingMode }
	function FromName(self, name: string): EnumDeviceSimulatorScalingMode?
	function FromValue(self, value: number): EnumDeviceSimulatorScalingMode?
end

declare extern type EnumDeviceType extends EnumItem with
end

declare extern type EnumDeviceType_INTERNAL extends Enum with
	Unknown: EnumDeviceType
	Desktop: EnumDeviceType
	Tablet: EnumDeviceType
	Phone: EnumDeviceType
	TV: EnumDeviceType
	function GetEnumItems(self): { EnumDeviceType }
	function FromName(self, name: string): EnumDeviceType?
	function FromValue(self, value: number): EnumDeviceType?
end

declare extern type EnumDialogBehaviorType extends EnumItem with
end

declare extern type EnumDialogBehaviorType_INTERNAL extends Enum with
	SinglePlayer: EnumDialogBehaviorType
	MultiplePlayers: EnumDialogBehaviorType
	function GetEnumItems(self): { EnumDialogBehaviorType }
	function FromName(self, name: string): EnumDialogBehaviorType?
	function FromValue(self, value: number): EnumDialogBehaviorType?
end

declare extern type EnumDialogPurpose extends EnumItem with
end

declare extern type EnumDialogPurpose_INTERNAL extends Enum with
	Quest: EnumDialogPurpose
	Help: EnumDialogPurpose
	Shop: EnumDialogPurpose
	function GetEnumItems(self): { EnumDialogPurpose }
	function FromName(self, name: string): EnumDialogPurpose?
	function FromValue(self, value: number): EnumDialogPurpose?
end

declare extern type EnumDialogTone extends EnumItem with
end

declare extern type EnumDialogTone_INTERNAL extends Enum with
	Neutral: EnumDialogTone
	Friendly: EnumDialogTone
	Enemy: EnumDialogTone
	function GetEnumItems(self): { EnumDialogTone }
	function FromName(self, name: string): EnumDialogTone?
	function FromValue(self, value: number): EnumDialogTone?
end

declare extern type EnumDigitsRigDescriptionSide extends EnumItem with
end

declare extern type EnumDigitsRigDescriptionSide_INTERNAL extends Enum with
	None: EnumDigitsRigDescriptionSide
	Left: EnumDigitsRigDescriptionSide
	Right: EnumDigitsRigDescriptionSide
	function GetEnumItems(self): { EnumDigitsRigDescriptionSide }
	function FromName(self, name: string): EnumDigitsRigDescriptionSide?
	function FromValue(self, value: number): EnumDigitsRigDescriptionSide?
end

declare extern type EnumDiscountType extends EnumItem with
end

declare extern type EnumDiscountType_INTERNAL extends Enum with
	Uncategorized: EnumDiscountType
	function GetEnumItems(self): { EnumDiscountType }
	function FromName(self, name: string): EnumDiscountType?
	function FromValue(self, value: number): EnumDiscountType?
end

declare extern type EnumDisplayScalingMode extends EnumItem with
end

declare extern type EnumDisplayScalingMode_INTERNAL extends Enum with
	Default: EnumDisplayScalingMode
	Legacy: EnumDisplayScalingMode
	Responsive: EnumDisplayScalingMode
	function GetEnumItems(self): { EnumDisplayScalingMode }
	function FromName(self, name: string): EnumDisplayScalingMode?
	function FromValue(self, value: number): EnumDisplayScalingMode?
end

declare extern type EnumDisplaySize extends EnumItem with
end

declare extern type EnumDisplaySize_INTERNAL extends Enum with
	Small: EnumDisplaySize
	Medium: EnumDisplaySize
	Large: EnumDisplaySize
	function GetEnumItems(self): { EnumDisplaySize }
	function FromName(self, name: string): EnumDisplaySize?
	function FromValue(self, value: number): EnumDisplaySize?
end

declare extern type EnumDomainType extends EnumItem with
end

declare extern type EnumDomainType_INTERNAL extends Enum with
	EXPERIENCE: EnumDomainType
	OAUTH: EnumDomainType
	function GetEnumItems(self): { EnumDomainType }
	function FromName(self, name: string): EnumDomainType?
	function FromValue(self, value: number): EnumDomainType?
end

declare extern type EnumDominantAxis extends EnumItem with
end

declare extern type EnumDominantAxis_INTERNAL extends Enum with
	Width: EnumDominantAxis
	Height: EnumDominantAxis
	function GetEnumItems(self): { EnumDominantAxis }
	function FromName(self, name: string): EnumDominantAxis?
	function FromValue(self, value: number): EnumDominantAxis?
end

declare extern type EnumDraftStatusCode extends EnumItem with
end

declare extern type EnumDraftStatusCode_INTERNAL extends Enum with
	OK: EnumDraftStatusCode
	DraftOutdated: EnumDraftStatusCode
	ScriptRemoved: EnumDraftStatusCode
	DraftCommitted: EnumDraftStatusCode
	function GetEnumItems(self): { EnumDraftStatusCode }
	function FromName(self, name: string): EnumDraftStatusCode?
	function FromValue(self, value: number): EnumDraftStatusCode?
end

declare extern type EnumDragDetectorDragStyle extends EnumItem with
end

declare extern type EnumDragDetectorDragStyle_INTERNAL extends Enum with
	TranslateLine: EnumDragDetectorDragStyle
	TranslatePlane: EnumDragDetectorDragStyle
	TranslatePlaneOrLine: EnumDragDetectorDragStyle
	TranslateLineOrPlane: EnumDragDetectorDragStyle
	TranslateViewPlane: EnumDragDetectorDragStyle
	RotateAxis: EnumDragDetectorDragStyle
	RotateTrackball: EnumDragDetectorDragStyle
	Scriptable: EnumDragDetectorDragStyle
	BestForDevice: EnumDragDetectorDragStyle
	function GetEnumItems(self): { EnumDragDetectorDragStyle }
	function FromName(self, name: string): EnumDragDetectorDragStyle?
	function FromValue(self, value: number): EnumDragDetectorDragStyle?
end

declare extern type EnumDragDetectorPermissionPolicy extends EnumItem with
end

declare extern type EnumDragDetectorPermissionPolicy_INTERNAL extends Enum with
	Nobody: EnumDragDetectorPermissionPolicy
	Everybody: EnumDragDetectorPermissionPolicy
	Scriptable: EnumDragDetectorPermissionPolicy
	function GetEnumItems(self): { EnumDragDetectorPermissionPolicy }
	function FromName(self, name: string): EnumDragDetectorPermissionPolicy?
	function FromValue(self, value: number): EnumDragDetectorPermissionPolicy?
end

declare extern type EnumDragDetectorResponseStyle extends EnumItem with
end

declare extern type EnumDragDetectorResponseStyle_INTERNAL extends Enum with
	Geometric: EnumDragDetectorResponseStyle
	Physical: EnumDragDetectorResponseStyle
	Custom: EnumDragDetectorResponseStyle
	function GetEnumItems(self): { EnumDragDetectorResponseStyle }
	function FromName(self, name: string): EnumDragDetectorResponseStyle?
	function FromValue(self, value: number): EnumDragDetectorResponseStyle?
end

declare extern type EnumDraggerCoordinateSpace extends EnumItem with
end

declare extern type EnumDraggerCoordinateSpace_INTERNAL extends Enum with
	Object: EnumDraggerCoordinateSpace
	World: EnumDraggerCoordinateSpace
	function GetEnumItems(self): { EnumDraggerCoordinateSpace }
	function FromName(self, name: string): EnumDraggerCoordinateSpace?
	function FromValue(self, value: number): EnumDraggerCoordinateSpace?
end

declare extern type EnumDraggerMovementMode extends EnumItem with
end

declare extern type EnumDraggerMovementMode_INTERNAL extends Enum with
	Geometric: EnumDraggerMovementMode
	Physical: EnumDraggerMovementMode
	function GetEnumItems(self): { EnumDraggerMovementMode }
	function FromName(self, name: string): EnumDraggerMovementMode?
	function FromValue(self, value: number): EnumDraggerMovementMode?
end

declare extern type EnumDraggingScrollBar extends EnumItem with
end

declare extern type EnumDraggingScrollBar_INTERNAL extends Enum with
	None: EnumDraggingScrollBar
	Horizontal: EnumDraggingScrollBar
	Vertical: EnumDraggingScrollBar
	function GetEnumItems(self): { EnumDraggingScrollBar }
	function FromName(self, name: string): EnumDraggingScrollBar?
	function FromValue(self, value: number): EnumDraggingScrollBar?
end

declare extern type EnumEasingDirection extends EnumItem with
end

declare extern type EnumEasingDirection_INTERNAL extends Enum with
	In: EnumEasingDirection
	Out: EnumEasingDirection
	InOut: EnumEasingDirection
	function GetEnumItems(self): { EnumEasingDirection }
	function FromName(self, name: string): EnumEasingDirection?
	function FromValue(self, value: number): EnumEasingDirection?
end

declare extern type EnumEasingStyle extends EnumItem with
end

declare extern type EnumEasingStyle_INTERNAL extends Enum with
	Linear: EnumEasingStyle
	Sine: EnumEasingStyle
	Back: EnumEasingStyle
	Quad: EnumEasingStyle
	Quart: EnumEasingStyle
	Quint: EnumEasingStyle
	Bounce: EnumEasingStyle
	Elastic: EnumEasingStyle
	Exponential: EnumEasingStyle
	Circular: EnumEasingStyle
	Cubic: EnumEasingStyle
	function GetEnumItems(self): { EnumEasingStyle }
	function FromName(self, name: string): EnumEasingStyle?
	function FromValue(self, value: number): EnumEasingStyle?
end

declare extern type EnumEditableStatus extends EnumItem with
end

declare extern type EnumEditableStatus_INTERNAL extends Enum with
	Unknown: EnumEditableStatus
	Allowed: EnumEditableStatus
	Disallowed: EnumEditableStatus
	function GetEnumItems(self): { EnumEditableStatus }
	function FromName(self, name: string): EnumEditableStatus?
	function FromValue(self, value: number): EnumEditableStatus?
end

declare extern type EnumElasticBehavior extends EnumItem with
end

declare extern type EnumElasticBehavior_INTERNAL extends Enum with
	WhenScrollable: EnumElasticBehavior
	Always: EnumElasticBehavior
	Never: EnumElasticBehavior
	function GetEnumItems(self): { EnumElasticBehavior }
	function FromName(self, name: string): EnumElasticBehavior?
	function FromValue(self, value: number): EnumElasticBehavior?
end

declare extern type EnumEmitterPositionType extends EnumItem with
end

declare extern type EnumEmitterPositionType_INTERNAL extends Enum with
	Parent: EnumEmitterPositionType
	Instance: EnumEmitterPositionType
	function GetEnumItems(self): { EnumEmitterPositionType }
	function FromName(self, name: string): EnumEmitterPositionType?
	function FromValue(self, value: number): EnumEmitterPositionType?
end

declare extern type EnumEngagementLevel extends EnumItem with
end

declare extern type EnumEngagementLevel_INTERNAL extends Enum with
	Unknown: EnumEngagementLevel
	Inactive: EnumEngagementLevel
	Low: EnumEngagementLevel
	Medium: EnumEngagementLevel
	High: EnumEngagementLevel
	function GetEnumItems(self): { EnumEngagementLevel }
	function FromName(self, name: string): EnumEngagementLevel?
	function FromValue(self, value: number): EnumEngagementLevel?
end

declare extern type EnumEngineFolder extends EnumItem with
end

declare extern type EnumEngineFolder_INTERNAL extends Enum with
	Screenshots: EnumEngineFolder
	Videos: EnumEngineFolder
	Logs: EnumEngineFolder
	function GetEnumItems(self): { EnumEngineFolder }
	function FromName(self, name: string): EnumEngineFolder?
	function FromValue(self, value: number): EnumEngineFolder?
end

declare extern type EnumEnviromentalPhysicsThrottle extends EnumItem with
end

declare extern type EnumEnviromentalPhysicsThrottle_INTERNAL extends Enum with
	DefaultAuto: EnumEnviromentalPhysicsThrottle
	Disabled: EnumEnviromentalPhysicsThrottle
	Always: EnumEnviromentalPhysicsThrottle
	Skip2: EnumEnviromentalPhysicsThrottle
	Skip4: EnumEnviromentalPhysicsThrottle
	Skip8: EnumEnviromentalPhysicsThrottle
	Skip16: EnumEnviromentalPhysicsThrottle
	function GetEnumItems(self): { EnumEnviromentalPhysicsThrottle }
	function FromName(self, name: string): EnumEnviromentalPhysicsThrottle?
	function FromValue(self, value: number): EnumEnviromentalPhysicsThrottle?
end

declare extern type EnumExperienceActivationStatus extends EnumItem with
end

declare extern type EnumExperienceActivationStatus_INTERNAL extends Enum with
	Unknown: EnumExperienceActivationStatus
	New: EnumExperienceActivationStatus
	Active: EnumExperienceActivationStatus
	Lapsed: EnumExperienceActivationStatus
	Reactivated: EnumExperienceActivationStatus
	function GetEnumItems(self): { EnumExperienceActivationStatus }
	function FromName(self, name: string): EnumExperienceActivationStatus?
	function FromValue(self, value: number): EnumExperienceActivationStatus?
end

declare extern type EnumExperienceAuthScope extends EnumItem with
end

declare extern type EnumExperienceAuthScope_INTERNAL extends Enum with
	DefaultScope: EnumExperienceAuthScope
	CreatorAssetsCreate: EnumExperienceAuthScope
	function GetEnumItems(self): { EnumExperienceAuthScope }
	function FromName(self, name: string): EnumExperienceAuthScope?
	function FromValue(self, value: number): EnumExperienceAuthScope?
end

declare extern type EnumExperienceEventStatus extends EnumItem with
end

declare extern type EnumExperienceEventStatus_INTERNAL extends Enum with
	Active: EnumExperienceEventStatus
	Cancelled: EnumExperienceEventStatus
	Moderated: EnumExperienceEventStatus
	Unpublished: EnumExperienceEventStatus
	Unknown: EnumExperienceEventStatus
	function GetEnumItems(self): { EnumExperienceEventStatus }
	function FromName(self, name: string): EnumExperienceEventStatus?
	function FromValue(self, value: number): EnumExperienceEventStatus?
end

declare extern type EnumExperienceStateCaptureSelectionMode extends EnumItem with
end

declare extern type EnumExperienceStateCaptureSelectionMode_INTERNAL extends Enum with
	Default: EnumExperienceStateCaptureSelectionMode
	SafetyHighlightMode: EnumExperienceStateCaptureSelectionMode
	function GetEnumItems(self): { EnumExperienceStateCaptureSelectionMode }
	function FromName(self, name: string): EnumExperienceStateCaptureSelectionMode?
	function FromValue(self, value: number): EnumExperienceStateCaptureSelectionMode?
end

declare extern type EnumExperienceStateRecordingLoadMode extends EnumItem with
end

declare extern type EnumExperienceStateRecordingLoadMode_INTERNAL extends Enum with
	NewReplay: EnumExperienceStateRecordingLoadMode
	ContiguousSlice: EnumExperienceStateRecordingLoadMode
	NoncontiguousSlice: EnumExperienceStateRecordingLoadMode
	function GetEnumItems(self): { EnumExperienceStateRecordingLoadMode }
	function FromName(self, name: string): EnumExperienceStateRecordingLoadMode?
	function FromValue(self, value: number): EnumExperienceStateRecordingLoadMode?
end

declare extern type EnumExperienceStateRecordingLoadSourceType extends EnumItem with
end

declare extern type EnumExperienceStateRecordingLoadSourceType_INTERNAL extends Enum with
	S3Url: EnumExperienceStateRecordingLoadSourceType
	File: EnumExperienceStateRecordingLoadSourceType
	function GetEnumItems(self): { EnumExperienceStateRecordingLoadSourceType }
	function FromName(self, name: string): EnumExperienceStateRecordingLoadSourceType?
	function FromValue(self, value: number): EnumExperienceStateRecordingLoadSourceType?
end

declare extern type EnumExperienceStateRecordingPlaybackMode extends EnumItem with
end

declare extern type EnumExperienceStateRecordingPlaybackMode_INTERNAL extends Enum with
	Undefined: EnumExperienceStateRecordingPlaybackMode
	Stopped: EnumExperienceStateRecordingPlaybackMode
	Playing: EnumExperienceStateRecordingPlaybackMode
	Rewinding: EnumExperienceStateRecordingPlaybackMode
	function GetEnumItems(self): { EnumExperienceStateRecordingPlaybackMode }
	function FromName(self, name: string): EnumExperienceStateRecordingPlaybackMode?
	function FromValue(self, value: number): EnumExperienceStateRecordingPlaybackMode?
end

declare extern type EnumExplosionType extends EnumItem with
end

declare extern type EnumExplosionType_INTERNAL extends Enum with
	NoCraters: EnumExplosionType
	Craters: EnumExplosionType
	function GetEnumItems(self): { EnumExplosionType }
	function FromName(self, name: string): EnumExplosionType?
	function FromValue(self, value: number): EnumExplosionType?
end

declare extern type EnumExternalEditorMode extends EnumItem with
end

declare extern type EnumExternalEditorMode_INTERNAL extends Enum with
	SystemDefault: EnumExternalEditorMode
	UserSelectedEditor: EnumExternalEditorMode
	function GetEnumItems(self): { EnumExternalEditorMode }
	function FromName(self, name: string): EnumExternalEditorMode?
	function FromValue(self, value: number): EnumExternalEditorMode?
end

declare extern type EnumFACSDataLod extends EnumItem with
end

declare extern type EnumFACSDataLod_INTERNAL extends Enum with
	LOD0: EnumFACSDataLod
	LOD1: EnumFACSDataLod
	LODCount: EnumFACSDataLod
	function GetEnumItems(self): { EnumFACSDataLod }
	function FromName(self, name: string): EnumFACSDataLod?
	function FromValue(self, value: number): EnumFACSDataLod?
end

declare extern type EnumFacialAgeEstimationResultType extends EnumItem with
end

declare extern type EnumFacialAgeEstimationResultType_INTERNAL extends Enum with
	Complete: EnumFacialAgeEstimationResultType
	Cancel: EnumFacialAgeEstimationResultType
	Error: EnumFacialAgeEstimationResultType
	function GetEnumItems(self): { EnumFacialAgeEstimationResultType }
	function FromName(self, name: string): EnumFacialAgeEstimationResultType?
	function FromValue(self, value: number): EnumFacialAgeEstimationResultType?
end

declare extern type EnumFacialAnimationStreamingState extends EnumItem with
end

declare extern type EnumFacialAnimationStreamingState_INTERNAL extends Enum with
	None: EnumFacialAnimationStreamingState
	Audio: EnumFacialAnimationStreamingState
	Video: EnumFacialAnimationStreamingState
	Place: EnumFacialAnimationStreamingState
	Server: EnumFacialAnimationStreamingState
	function GetEnumItems(self): { EnumFacialAnimationStreamingState }
	function FromName(self, name: string): EnumFacialAnimationStreamingState?
	function FromValue(self, value: number): EnumFacialAnimationStreamingState?
end

declare extern type EnumFacsActionUnit extends EnumItem with
end

declare extern type EnumFacsActionUnit_INTERNAL extends Enum with
	ChinRaiserUpperLip: EnumFacsActionUnit
	ChinRaiser: EnumFacsActionUnit
	FlatPucker: EnumFacsActionUnit
	Funneler: EnumFacsActionUnit
	LowerLipSuck: EnumFacsActionUnit
	LipPresser: EnumFacsActionUnit
	LipsTogether: EnumFacsActionUnit
	MouthLeft: EnumFacsActionUnit
	MouthRight: EnumFacsActionUnit
	Pucker: EnumFacsActionUnit
	UpperLipSuck: EnumFacsActionUnit
	LeftCheekPuff: EnumFacsActionUnit
	LeftDimpler: EnumFacsActionUnit
	LeftLipCornerDown: EnumFacsActionUnit
	LeftLowerLipDepressor: EnumFacsActionUnit
	LeftLipCornerPuller: EnumFacsActionUnit
	LeftLipStretcher: EnumFacsActionUnit
	LeftUpperLipRaiser: EnumFacsActionUnit
	RightCheekPuff: EnumFacsActionUnit
	RightDimpler: EnumFacsActionUnit
	RightLipCornerDown: EnumFacsActionUnit
	RightLowerLipDepressor: EnumFacsActionUnit
	RightLipCornerPuller: EnumFacsActionUnit
	RightLipStretcher: EnumFacsActionUnit
	RightUpperLipRaiser: EnumFacsActionUnit
	JawDrop: EnumFacsActionUnit
	JawLeft: EnumFacsActionUnit
	JawRight: EnumFacsActionUnit
	Corrugator: EnumFacsActionUnit
	LeftBrowLowerer: EnumFacsActionUnit
	LeftOuterBrowRaiser: EnumFacsActionUnit
	LeftNoseWrinkler: EnumFacsActionUnit
	LeftInnerBrowRaiser: EnumFacsActionUnit
	RightBrowLowerer: EnumFacsActionUnit
	RightOuterBrowRaiser: EnumFacsActionUnit
	RightInnerBrowRaiser: EnumFacsActionUnit
	RightNoseWrinkler: EnumFacsActionUnit
	EyesLookDown: EnumFacsActionUnit
	EyesLookLeft: EnumFacsActionUnit
	EyesLookUp: EnumFacsActionUnit
	EyesLookRight: EnumFacsActionUnit
	LeftCheekRaiser: EnumFacsActionUnit
	LeftEyeUpperLidRaiser: EnumFacsActionUnit
	LeftEyeClosed: EnumFacsActionUnit
	RightCheekRaiser: EnumFacsActionUnit
	RightEyeUpperLidRaiser: EnumFacsActionUnit
	RightEyeClosed: EnumFacsActionUnit
	TongueDown: EnumFacsActionUnit
	TongueOut: EnumFacsActionUnit
	TongueUp: EnumFacsActionUnit
	function GetEnumItems(self): { EnumFacsActionUnit }
	function FromName(self, name: string): EnumFacsActionUnit?
	function FromValue(self, value: number): EnumFacsActionUnit?
end

declare extern type EnumFeatureRestrictionAbuseVector extends EnumItem with
end

declare extern type EnumFeatureRestrictionAbuseVector_INTERNAL extends Enum with
	ExperienceChat: EnumFeatureRestrictionAbuseVector
	Communication: EnumFeatureRestrictionAbuseVector
	function GetEnumItems(self): { EnumFeatureRestrictionAbuseVector }
	function FromName(self, name: string): EnumFeatureRestrictionAbuseVector?
	function FromValue(self, value: number): EnumFeatureRestrictionAbuseVector?
end

declare extern type EnumFeedbackType extends EnumItem with
end

declare extern type EnumFeedbackType_INTERNAL extends Enum with
	Feedback: EnumFeedbackType
	PlayerSupport: EnumFeedbackType
	function GetEnumItems(self): { EnumFeedbackType }
	function FromName(self, name: string): EnumFeedbackType?
	function FromValue(self, value: number): EnumFeedbackType?
end

declare extern type EnumFieldOfViewMode extends EnumItem with
end

declare extern type EnumFieldOfViewMode_INTERNAL extends Enum with
	Vertical: EnumFieldOfViewMode
	Diagonal: EnumFieldOfViewMode
	MaxAxis: EnumFieldOfViewMode
	function GetEnumItems(self): { EnumFieldOfViewMode }
	function FromName(self, name: string): EnumFieldOfViewMode?
	function FromValue(self, value: number): EnumFieldOfViewMode?
end

declare extern type EnumFillDirection extends EnumItem with
end

declare extern type EnumFillDirection_INTERNAL extends Enum with
	Horizontal: EnumFillDirection
	Vertical: EnumFillDirection
	function GetEnumItems(self): { EnumFillDirection }
	function FromName(self, name: string): EnumFillDirection?
	function FromValue(self, value: number): EnumFillDirection?
end

declare extern type EnumFilterErrorType extends EnumItem with
end

declare extern type EnumFilterErrorType_INTERNAL extends Enum with
	BackslashNotEscapingAnything: EnumFilterErrorType
	BadBespokeFilter: EnumFilterErrorType
	BadName: EnumFilterErrorType
	IncompleteOr: EnumFilterErrorType
	IncompleteParenthesis: EnumFilterErrorType
	InvalidDoubleStar: EnumFilterErrorType
	InvalidTilde: EnumFilterErrorType
	PropertyBadOperator: EnumFilterErrorType
	PropertyDoesNotExist: EnumFilterErrorType
	PropertyInvalidField: EnumFilterErrorType
	PropertyInvalidValue: EnumFilterErrorType
	PropertyUnsupportedFields: EnumFilterErrorType
	PropertyUnsupportedProperty: EnumFilterErrorType
	UnexpectedNameIndex: EnumFilterErrorType
	UnexpectedToken: EnumFilterErrorType
	UnfinishedBinaryOperator: EnumFilterErrorType
	UnfinishedQuote: EnumFilterErrorType
	UnknownBespokeFilter: EnumFilterErrorType
	WildcardInProperty: EnumFilterErrorType
	function GetEnumItems(self): { EnumFilterErrorType }
	function FromName(self, name: string): EnumFilterErrorType?
	function FromValue(self, value: number): EnumFilterErrorType?
end

declare extern type EnumFilterResult extends EnumItem with
end

declare extern type EnumFilterResult_INTERNAL extends Enum with
	Accepted: EnumFilterResult
	Rejected: EnumFilterResult
	function GetEnumItems(self): { EnumFilterResult }
	function FromName(self, name: string): EnumFilterResult?
	function FromValue(self, value: number): EnumFilterResult?
end

declare extern type EnumFilterType extends EnumItem with
end

declare extern type EnumFilterType_INTERNAL extends Enum with
	Exclude: EnumFilterType
	Include: EnumFilterType
	function GetEnumItems(self): { EnumFilterType }
	function FromName(self, name: string): EnumFilterType?
	function FromValue(self, value: number): EnumFilterType?
end

declare extern type EnumFinishRecordingOperation extends EnumItem with
end

declare extern type EnumFinishRecordingOperation_INTERNAL extends Enum with
	Cancel: EnumFinishRecordingOperation
	Commit: EnumFinishRecordingOperation
	Append: EnumFinishRecordingOperation
	function GetEnumItems(self): { EnumFinishRecordingOperation }
	function FromName(self, name: string): EnumFinishRecordingOperation?
	function FromValue(self, value: number): EnumFinishRecordingOperation?
end

declare extern type EnumFluidFidelity extends EnumItem with
end

declare extern type EnumFluidFidelity_INTERNAL extends Enum with
	Automatic: EnumFluidFidelity
	UseCollisionGeometry: EnumFluidFidelity
	UsePreciseGeometry: EnumFluidFidelity
	function GetEnumItems(self): { EnumFluidFidelity }
	function FromName(self, name: string): EnumFluidFidelity?
	function FromValue(self, value: number): EnumFluidFidelity?
end

declare extern type EnumFluidForces extends EnumItem with
end

declare extern type EnumFluidForces_INTERNAL extends Enum with
	Default: EnumFluidForces
	Experimental: EnumFluidForces
	function GetEnumItems(self): { EnumFluidForces }
	function FromName(self, name: string): EnumFluidForces?
	function FromValue(self, value: number): EnumFluidForces?
end

declare extern type EnumFont extends EnumItem with
end

declare extern type EnumFont_INTERNAL extends Enum with
	Legacy: EnumFont
	Arial: EnumFont
	ArialBold: EnumFont
	SourceSans: EnumFont
	SourceSansBold: EnumFont
	SourceSansLight: EnumFont
	SourceSansItalic: EnumFont
	Bodoni: EnumFont
	Garamond: EnumFont
	Cartoon: EnumFont
	Code: EnumFont
	Highway: EnumFont
	SciFi: EnumFont
	Arcade: EnumFont
	Fantasy: EnumFont
	Antique: EnumFont
	SourceSansSemibold: EnumFont
	Gotham: EnumFont
	GothamMedium: EnumFont
	GothamBold: EnumFont
	GothamBlack: EnumFont
	AmaticSC: EnumFont
	Bangers: EnumFont
	Creepster: EnumFont
	DenkOne: EnumFont
	Fondamento: EnumFont
	FredokaOne: EnumFont
	GrenzeGotisch: EnumFont
	IndieFlower: EnumFont
	JosefinSans: EnumFont
	Jura: EnumFont
	Kalam: EnumFont
	LuckiestGuy: EnumFont
	Merriweather: EnumFont
	Michroma: EnumFont
	Nunito: EnumFont
	Oswald: EnumFont
	PatrickHand: EnumFont
	PermanentMarker: EnumFont
	Roboto: EnumFont
	RobotoCondensed: EnumFont
	RobotoMono: EnumFont
	Sarpanch: EnumFont
	SpecialElite: EnumFont
	TitilliumWeb: EnumFont
	Ubuntu: EnumFont
	BuilderSans: EnumFont
	BuilderSansMedium: EnumFont
	BuilderSansBold: EnumFont
	BuilderSansExtraBold: EnumFont
	Arimo: EnumFont
	ArimoBold: EnumFont
	Unknown: EnumFont
	function GetEnumItems(self): { EnumFont }
	function FromName(self, name: string): EnumFont?
	function FromValue(self, value: number): EnumFont?
end

declare extern type EnumFontSize extends EnumItem with
end

declare extern type EnumFontSize_INTERNAL extends Enum with
	Size8: EnumFontSize
	Size9: EnumFontSize
	Size10: EnumFontSize
	Size11: EnumFontSize
	Size12: EnumFontSize
	Size14: EnumFontSize
	Size18: EnumFontSize
	Size24: EnumFontSize
	Size36: EnumFontSize
	Size48: EnumFontSize
	Size28: EnumFontSize
	Size32: EnumFontSize
	Size42: EnumFontSize
	Size60: EnumFontSize
	Size96: EnumFontSize
	function GetEnumItems(self): { EnumFontSize }
	function FromName(self, name: string): EnumFontSize?
	function FromValue(self, value: number): EnumFontSize?
end

declare extern type EnumFontStyle extends EnumItem with
end

declare extern type EnumFontStyle_INTERNAL extends Enum with
	Normal: EnumFontStyle
	Italic: EnumFontStyle
	function GetEnumItems(self): { EnumFontStyle }
	function FromName(self, name: string): EnumFontStyle?
	function FromValue(self, value: number): EnumFontStyle?
end

declare extern type EnumFontWeight extends EnumItem with
end

declare extern type EnumFontWeight_INTERNAL extends Enum with
	Thin: EnumFontWeight
	ExtraLight: EnumFontWeight
	Light: EnumFontWeight
	Regular: EnumFontWeight
	Medium: EnumFontWeight
	SemiBold: EnumFontWeight
	Bold: EnumFontWeight
	ExtraBold: EnumFontWeight
	Heavy: EnumFontWeight
	function GetEnumItems(self): { EnumFontWeight }
	function FromName(self, name: string): EnumFontWeight?
	function FromValue(self, value: number): EnumFontWeight?
end

declare extern type EnumForceLimitMode extends EnumItem with
end

declare extern type EnumForceLimitMode_INTERNAL extends Enum with
	Magnitude: EnumForceLimitMode
	PerAxis: EnumForceLimitMode
	function GetEnumItems(self): { EnumForceLimitMode }
	function FromName(self, name: string): EnumForceLimitMode?
	function FromValue(self, value: number): EnumForceLimitMode?
end

declare extern type EnumFormFactor extends EnumItem with
end

declare extern type EnumFormFactor_INTERNAL extends Enum with
	Symmetric: EnumFormFactor
	Brick: EnumFormFactor
	Plate: EnumFormFactor
	Custom: EnumFormFactor
	function GetEnumItems(self): { EnumFormFactor }
	function FromName(self, name: string): EnumFormFactor?
	function FromValue(self, value: number): EnumFormFactor?
end

declare extern type EnumFrameStyle extends EnumItem with
end

declare extern type EnumFrameStyle_INTERNAL extends Enum with
	Custom: EnumFrameStyle
	ChatBlue: EnumFrameStyle
	RobloxSquare: EnumFrameStyle
	RobloxRound: EnumFrameStyle
	ChatGreen: EnumFrameStyle
	ChatRed: EnumFrameStyle
	DropShadow: EnumFrameStyle
	function GetEnumItems(self): { EnumFrameStyle }
	function FromName(self, name: string): EnumFrameStyle?
	function FromValue(self, value: number): EnumFrameStyle?
end

declare extern type EnumFramerateManagerMode extends EnumItem with
end

declare extern type EnumFramerateManagerMode_INTERNAL extends Enum with
	Automatic: EnumFramerateManagerMode
	On: EnumFramerateManagerMode
	Off: EnumFramerateManagerMode
	function GetEnumItems(self): { EnumFramerateManagerMode }
	function FromName(self, name: string): EnumFramerateManagerMode?
	function FromValue(self, value: number): EnumFramerateManagerMode?
end

declare extern type EnumFriendRequestEvent extends EnumItem with
end

declare extern type EnumFriendRequestEvent_INTERNAL extends Enum with
	Issue: EnumFriendRequestEvent
	Revoke: EnumFriendRequestEvent
	Accept: EnumFriendRequestEvent
	Deny: EnumFriendRequestEvent
	function GetEnumItems(self): { EnumFriendRequestEvent }
	function FromName(self, name: string): EnumFriendRequestEvent?
	function FromValue(self, value: number): EnumFriendRequestEvent?
end

declare extern type EnumFriendStatus extends EnumItem with
end

declare extern type EnumFriendStatus_INTERNAL extends Enum with
	Unknown: EnumFriendStatus
	NotFriend: EnumFriendStatus
	Friend: EnumFriendStatus
	FriendRequestSent: EnumFriendStatus
	FriendRequestReceived: EnumFriendStatus
	function GetEnumItems(self): { EnumFriendStatus }
	function FromName(self, name: string): EnumFriendStatus?
	function FromValue(self, value: number): EnumFriendStatus?
end

declare extern type EnumFunctionalTestResult extends EnumItem with
end

declare extern type EnumFunctionalTestResult_INTERNAL extends Enum with
	Passed: EnumFunctionalTestResult
	Warning: EnumFunctionalTestResult
	Error: EnumFunctionalTestResult
	function GetEnumItems(self): { EnumFunctionalTestResult }
	function FromName(self, name: string): EnumFunctionalTestResult?
	function FromValue(self, value: number): EnumFunctionalTestResult?
end

declare extern type EnumGameAvatarType extends EnumItem with
end

declare extern type EnumGameAvatarType_INTERNAL extends Enum with
	R6: EnumGameAvatarType
	R15: EnumGameAvatarType
	PlayerChoice: EnumGameAvatarType
	function GetEnumItems(self): { EnumGameAvatarType }
	function FromName(self, name: string): EnumGameAvatarType?
	function FromValue(self, value: number): EnumGameAvatarType?
end

declare extern type EnumGamepadType extends EnumItem with
end

declare extern type EnumGamepadType_INTERNAL extends Enum with
	Unknown: EnumGamepadType
	PS4: EnumGamepadType
	PS5: EnumGamepadType
	XboxOne: EnumGamepadType
	function GetEnumItems(self): { EnumGamepadType }
	function FromName(self, name: string): EnumGamepadType?
	function FromValue(self, value: number): EnumGamepadType?
end

declare extern type EnumGearGenreSetting extends EnumItem with
end

declare extern type EnumGearGenreSetting_INTERNAL extends Enum with
	AllGenres: EnumGearGenreSetting
	MatchingGenreOnly: EnumGearGenreSetting
	function GetEnumItems(self): { EnumGearGenreSetting }
	function FromName(self, name: string): EnumGearGenreSetting?
	function FromValue(self, value: number): EnumGearGenreSetting?
end

declare extern type EnumGearType extends EnumItem with
end

declare extern type EnumGearType_INTERNAL extends Enum with
	MeleeWeapons: EnumGearType
	RangedWeapons: EnumGearType
	Explosives: EnumGearType
	PowerUps: EnumGearType
	NavigationEnhancers: EnumGearType
	MusicalInstruments: EnumGearType
	SocialItems: EnumGearType
	BuildingTools: EnumGearType
	Transport: EnumGearType
	function GetEnumItems(self): { EnumGearType }
	function FromName(self, name: string): EnumGearType?
	function FromValue(self, value: number): EnumGearType?
end

declare extern type EnumGenre extends EnumItem with
end

declare extern type EnumGenre_INTERNAL extends Enum with
	All: EnumGenre
	TownAndCity: EnumGenre
	Fantasy: EnumGenre
	SciFi: EnumGenre
	Ninja: EnumGenre
	Scary: EnumGenre
	Pirate: EnumGenre
	Adventure: EnumGenre
	Sports: EnumGenre
	Funny: EnumGenre
	WildWest: EnumGenre
	War: EnumGenre
	SkatePark: EnumGenre
	Tutorial: EnumGenre
	function GetEnumItems(self): { EnumGenre }
	function FromName(self, name: string): EnumGenre?
	function FromValue(self, value: number): EnumGenre?
end

declare extern type EnumGradientTileMode extends EnumItem with
end

declare extern type EnumGradientTileMode_INTERNAL extends Enum with
	Clamp: EnumGradientTileMode
	Repeat: EnumGradientTileMode
	Mirror: EnumGradientTileMode
	function GetEnumItems(self): { EnumGradientTileMode }
	function FromName(self, name: string): EnumGradientTileMode?
	function FromValue(self, value: number): EnumGradientTileMode?
end

declare extern type EnumGradientType extends EnumItem with
end

declare extern type EnumGradientType_INTERNAL extends Enum with
	Linear: EnumGradientType
	Radial: EnumGradientType
	Conical: EnumGradientType
	function GetEnumItems(self): { EnumGradientType }
	function FromName(self, name: string): EnumGradientType?
	function FromValue(self, value: number): EnumGradientType?
end

declare extern type EnumGraphicsMode extends EnumItem with
end

declare extern type EnumGraphicsMode_INTERNAL extends Enum with
	Automatic: EnumGraphicsMode
	Direct3D11: EnumGraphicsMode
	OpenGL: EnumGraphicsMode
	Metal: EnumGraphicsMode
	Vulkan: EnumGraphicsMode
	NoGraphics: EnumGraphicsMode
	function GetEnumItems(self): { EnumGraphicsMode }
	function FromName(self, name: string): EnumGraphicsMode?
	function FromValue(self, value: number): EnumGraphicsMode?
end

declare extern type EnumGraphicsOptimizationMode extends EnumItem with
end

declare extern type EnumGraphicsOptimizationMode_INTERNAL extends Enum with
	Performance: EnumGraphicsOptimizationMode
	Balanced: EnumGraphicsOptimizationMode
	Quality: EnumGraphicsOptimizationMode
	function GetEnumItems(self): { EnumGraphicsOptimizationMode }
	function FromName(self, name: string): EnumGraphicsOptimizationMode?
	function FromValue(self, value: number): EnumGraphicsOptimizationMode?
end

declare extern type EnumGroupMembershipStatus extends EnumItem with
end

declare extern type EnumGroupMembershipStatus_INTERNAL extends Enum with
	None: EnumGroupMembershipStatus
	Joined: EnumGroupMembershipStatus
	JoinRequestPending: EnumGroupMembershipStatus
	AlreadyMember: EnumGroupMembershipStatus
	function GetEnumItems(self): { EnumGroupMembershipStatus }
	function FromName(self, name: string): EnumGroupMembershipStatus?
	function FromValue(self, value: number): EnumGroupMembershipStatus?
end

declare extern type EnumGuiState extends EnumItem with
end

declare extern type EnumGuiState_INTERNAL extends Enum with
	Idle: EnumGuiState
	Hover: EnumGuiState
	Press: EnumGuiState
	NonInteractable: EnumGuiState
	function GetEnumItems(self): { EnumGuiState }
	function FromName(self, name: string): EnumGuiState?
	function FromValue(self, value: number): EnumGuiState?
end

declare extern type EnumGuiType extends EnumItem with
end

declare extern type EnumGuiType_INTERNAL extends Enum with
	Core: EnumGuiType
	Custom: EnumGuiType
	PlayerNameplates: EnumGuiType
	CustomBillboards: EnumGuiType
	CoreBillboards: EnumGuiType
	function GetEnumItems(self): { EnumGuiType }
	function FromName(self, name: string): EnumGuiType?
	function FromValue(self, value: number): EnumGuiType?
end

declare extern type EnumHandlesStyle extends EnumItem with
end

declare extern type EnumHandlesStyle_INTERNAL extends Enum with
	Resize: EnumHandlesStyle
	Movement: EnumHandlesStyle
	function GetEnumItems(self): { EnumHandlesStyle }
	function FromName(self, name: string): EnumHandlesStyle?
	function FromValue(self, value: number): EnumHandlesStyle?
end

declare extern type EnumHapticEffectType extends EnumItem with
end

declare extern type EnumHapticEffectType_INTERNAL extends Enum with
	Custom: EnumHapticEffectType
	UIHover: EnumHapticEffectType
	UIClick: EnumHapticEffectType
	UINotification: EnumHapticEffectType
	GameplayExplosion: EnumHapticEffectType
	GameplayCollision: EnumHapticEffectType
	function GetEnumItems(self): { EnumHapticEffectType }
	function FromName(self, name: string): EnumHapticEffectType?
	function FromValue(self, value: number): EnumHapticEffectType?
end

declare extern type EnumHashAlgorithm extends EnumItem with
end

declare extern type EnumHashAlgorithm_INTERNAL extends Enum with
	Blake2b: EnumHashAlgorithm
	Blake3: EnumHashAlgorithm
	Md5: EnumHashAlgorithm
	Sha1: EnumHashAlgorithm
	Sha256: EnumHashAlgorithm
	function GetEnumItems(self): { EnumHashAlgorithm }
	function FromName(self, name: string): EnumHashAlgorithm?
	function FromValue(self, value: number): EnumHashAlgorithm?
end

declare extern type EnumHighlightDepthMode extends EnumItem with
end

declare extern type EnumHighlightDepthMode_INTERNAL extends Enum with
	AlwaysOnTop: EnumHighlightDepthMode
	Occluded: EnumHighlightDepthMode
	function GetEnumItems(self): { EnumHighlightDepthMode }
	function FromName(self, name: string): EnumHighlightDepthMode?
	function FromValue(self, value: number): EnumHighlightDepthMode?
end

declare extern type EnumHorizontalAlignment extends EnumItem with
end

declare extern type EnumHorizontalAlignment_INTERNAL extends Enum with
	Center: EnumHorizontalAlignment
	Left: EnumHorizontalAlignment
	Right: EnumHorizontalAlignment
	function GetEnumItems(self): { EnumHorizontalAlignment }
	function FromName(self, name: string): EnumHorizontalAlignment?
	function FromValue(self, value: number): EnumHorizontalAlignment?
end

declare extern type EnumHoverAnimateSpeed extends EnumItem with
end

declare extern type EnumHoverAnimateSpeed_INTERNAL extends Enum with
	VerySlow: EnumHoverAnimateSpeed
	Slow: EnumHoverAnimateSpeed
	Medium: EnumHoverAnimateSpeed
	Fast: EnumHoverAnimateSpeed
	VeryFast: EnumHoverAnimateSpeed
	function GetEnumItems(self): { EnumHoverAnimateSpeed }
	function FromName(self, name: string): EnumHoverAnimateSpeed?
	function FromValue(self, value: number): EnumHoverAnimateSpeed?
end

declare extern type EnumHttpCachePolicy extends EnumItem with
end

declare extern type EnumHttpCachePolicy_INTERNAL extends Enum with
	None: EnumHttpCachePolicy
	Full: EnumHttpCachePolicy
	DataOnly: EnumHttpCachePolicy
	Default: EnumHttpCachePolicy
	InternalRedirectRefresh: EnumHttpCachePolicy
	function GetEnumItems(self): { EnumHttpCachePolicy }
	function FromName(self, name: string): EnumHttpCachePolicy?
	function FromValue(self, value: number): EnumHttpCachePolicy?
end

declare extern type EnumHttpCompression extends EnumItem with
end

declare extern type EnumHttpCompression_INTERNAL extends Enum with
	None: EnumHttpCompression
	Gzip: EnumHttpCompression
	function GetEnumItems(self): { EnumHttpCompression }
	function FromName(self, name: string): EnumHttpCompression?
	function FromValue(self, value: number): EnumHttpCompression?
end

declare extern type EnumHttpContentType extends EnumItem with
end

declare extern type EnumHttpContentType_INTERNAL extends Enum with
	ApplicationJson: EnumHttpContentType
	ApplicationXml: EnumHttpContentType
	ApplicationUrlEncoded: EnumHttpContentType
	TextPlain: EnumHttpContentType
	TextXml: EnumHttpContentType
	function GetEnumItems(self): { EnumHttpContentType }
	function FromName(self, name: string): EnumHttpContentType?
	function FromValue(self, value: number): EnumHttpContentType?
end

declare extern type EnumHttpError extends EnumItem with
end

declare extern type EnumHttpError_INTERNAL extends Enum with
	OK: EnumHttpError
	InvalidUrl: EnumHttpError
	DnsResolve: EnumHttpError
	ConnectFail: EnumHttpError
	OutOfMemory: EnumHttpError
	TimedOut: EnumHttpError
	TooManyRedirects: EnumHttpError
	InvalidRedirect: EnumHttpError
	NetFail: EnumHttpError
	Aborted: EnumHttpError
	SslConnectFail: EnumHttpError
	SslVerificationFail: EnumHttpError
	Unknown: EnumHttpError
	ConnectionClosed: EnumHttpError
	ServerProtocolError: EnumHttpError
	CreatorEnvironmentsNotSupportedByService: EnumHttpError
	InactivityTimeout: EnumHttpError
	TooManyOutstandingRequests: EnumHttpError
	InvalidRangeResponse: EnumHttpError
	function GetEnumItems(self): { EnumHttpError }
	function FromName(self, name: string): EnumHttpError?
	function FromValue(self, value: number): EnumHttpError?
end

declare extern type EnumHttpRequestType extends EnumItem with
end

declare extern type EnumHttpRequestType_INTERNAL extends Enum with
	Default: EnumHttpRequestType
	MarketplaceService: EnumHttpRequestType
	Players: EnumHttpRequestType
	Chat: EnumHttpRequestType
	Avatar: EnumHttpRequestType
	Analytics: EnumHttpRequestType
	Localization: EnumHttpRequestType
	function GetEnumItems(self): { EnumHttpRequestType }
	function FromName(self, name: string): EnumHttpRequestType?
	function FromValue(self, value: number): EnumHttpRequestType?
end

declare extern type EnumHumanoidCollisionType extends EnumItem with
end

declare extern type EnumHumanoidCollisionType_INTERNAL extends Enum with
	OuterBox: EnumHumanoidCollisionType
	InnerBox: EnumHumanoidCollisionType
	function GetEnumItems(self): { EnumHumanoidCollisionType }
	function FromName(self, name: string): EnumHumanoidCollisionType?
	function FromValue(self, value: number): EnumHumanoidCollisionType?
end

declare extern type EnumHumanoidDisplayDistanceType extends EnumItem with
end

declare extern type EnumHumanoidDisplayDistanceType_INTERNAL extends Enum with
	Viewer: EnumHumanoidDisplayDistanceType
	Subject: EnumHumanoidDisplayDistanceType
	None: EnumHumanoidDisplayDistanceType
	function GetEnumItems(self): { EnumHumanoidDisplayDistanceType }
	function FromName(self, name: string): EnumHumanoidDisplayDistanceType?
	function FromValue(self, value: number): EnumHumanoidDisplayDistanceType?
end

declare extern type EnumHumanoidHealthDisplayType extends EnumItem with
end

declare extern type EnumHumanoidHealthDisplayType_INTERNAL extends Enum with
	DisplayWhenDamaged: EnumHumanoidHealthDisplayType
	AlwaysOn: EnumHumanoidHealthDisplayType
	AlwaysOff: EnumHumanoidHealthDisplayType
	function GetEnumItems(self): { EnumHumanoidHealthDisplayType }
	function FromName(self, name: string): EnumHumanoidHealthDisplayType?
	function FromValue(self, value: number): EnumHumanoidHealthDisplayType?
end

declare extern type EnumHumanoidRigType extends EnumItem with
end

declare extern type EnumHumanoidRigType_INTERNAL extends Enum with
	R6: EnumHumanoidRigType
	R15: EnumHumanoidRigType
	function GetEnumItems(self): { EnumHumanoidRigType }
	function FromName(self, name: string): EnumHumanoidRigType?
	function FromValue(self, value: number): EnumHumanoidRigType?
end

declare extern type EnumHumanoidStateType extends EnumItem with
end

declare extern type EnumHumanoidStateType_INTERNAL extends Enum with
	FallingDown: EnumHumanoidStateType
	Ragdoll: EnumHumanoidStateType
	GettingUp: EnumHumanoidStateType
	Jumping: EnumHumanoidStateType
	Swimming: EnumHumanoidStateType
	Freefall: EnumHumanoidStateType
	Flying: EnumHumanoidStateType
	Landed: EnumHumanoidStateType
	Running: EnumHumanoidStateType
	RunningNoPhysics: EnumHumanoidStateType
	StrafingNoPhysics: EnumHumanoidStateType
	Climbing: EnumHumanoidStateType
	Seated: EnumHumanoidStateType
	PlatformStanding: EnumHumanoidStateType
	Dead: EnumHumanoidStateType
	Physics: EnumHumanoidStateType
	None: EnumHumanoidStateType
	function GetEnumItems(self): { EnumHumanoidStateType }
	function FromName(self, name: string): EnumHumanoidStateType?
	function FromValue(self, value: number): EnumHumanoidStateType?
end

declare extern type EnumIKCollisionsMode extends EnumItem with
end

declare extern type EnumIKCollisionsMode_INTERNAL extends Enum with
	NoCollisions: EnumIKCollisionsMode
	OtherMechanismsAnchored: EnumIKCollisionsMode
	IncludeContactedMechanisms: EnumIKCollisionsMode
	function GetEnumItems(self): { EnumIKCollisionsMode }
	function FromName(self, name: string): EnumIKCollisionsMode?
	function FromValue(self, value: number): EnumIKCollisionsMode?
end

declare extern type EnumIKControlConstraintSupport extends EnumItem with
end

declare extern type EnumIKControlConstraintSupport_INTERNAL extends Enum with
	Default: EnumIKControlConstraintSupport
	Disabled: EnumIKControlConstraintSupport
	Enabled: EnumIKControlConstraintSupport
	function GetEnumItems(self): { EnumIKControlConstraintSupport }
	function FromName(self, name: string): EnumIKControlConstraintSupport?
	function FromValue(self, value: number): EnumIKControlConstraintSupport?
end

declare extern type EnumIKControlType extends EnumItem with
end

declare extern type EnumIKControlType_INTERNAL extends Enum with
	Transform: EnumIKControlType
	Position: EnumIKControlType
	Rotation: EnumIKControlType
	LookAt: EnumIKControlType
	function GetEnumItems(self): { EnumIKControlType }
	function FromName(self, name: string): EnumIKControlType?
	function FromValue(self, value: number): EnumIKControlType?
end

declare extern type EnumIXPLoadingStatus extends EnumItem with
end

declare extern type EnumIXPLoadingStatus_INTERNAL extends Enum with
	None: EnumIXPLoadingStatus
	Pending: EnumIXPLoadingStatus
	Initialized: EnumIXPLoadingStatus
	ErrorInvalidUser: EnumIXPLoadingStatus
	ErrorConnection: EnumIXPLoadingStatus
	ErrorJsonParse: EnumIXPLoadingStatus
	ErrorTimedOut: EnumIXPLoadingStatus
	function GetEnumItems(self): { EnumIXPLoadingStatus }
	function FromName(self, name: string): EnumIXPLoadingStatus?
	function FromValue(self, value: number): EnumIXPLoadingStatus?
end

declare extern type EnumImageAlphaType extends EnumItem with
end

declare extern type EnumImageAlphaType_INTERNAL extends Enum with
	Default: EnumImageAlphaType
	LockCanvasAlpha: EnumImageAlphaType
	LockCanvasColor: EnumImageAlphaType
	function GetEnumItems(self): { EnumImageAlphaType }
	function FromName(self, name: string): EnumImageAlphaType?
	function FromValue(self, value: number): EnumImageAlphaType?
end

declare extern type EnumImageCombineType extends EnumItem with
end

declare extern type EnumImageCombineType_INTERNAL extends Enum with
	BlendSourceOver: EnumImageCombineType
	Overwrite: EnumImageCombineType
	Add: EnumImageCombineType
	Multiply: EnumImageCombineType
	AlphaBlend: EnumImageCombineType
	NormalMapBlend: EnumImageCombineType
	function GetEnumItems(self): { EnumImageCombineType }
	function FromName(self, name: string): EnumImageCombineType?
	function FromValue(self, value: number): EnumImageCombineType?
end

declare extern type EnumInOut extends EnumItem with
end

declare extern type EnumInOut_INTERNAL extends Enum with
	Edge: EnumInOut
	Inset: EnumInOut
	Center: EnumInOut
	function GetEnumItems(self): { EnumInOut }
	function FromName(self, name: string): EnumInOut?
	function FromValue(self, value: number): EnumInOut?
end

declare extern type EnumInfoType extends EnumItem with
end

declare extern type EnumInfoType_INTERNAL extends Enum with
	Asset: EnumInfoType
	Product: EnumInfoType
	GamePass: EnumInfoType
	Subscription: EnumInfoType
	Bundle: EnumInfoType
	function GetEnumItems(self): { EnumInfoType }
	function FromName(self, name: string): EnumInfoType?
	function FromValue(self, value: number): EnumInfoType?
end

declare extern type EnumInitialDockState extends EnumItem with
end

declare extern type EnumInitialDockState_INTERNAL extends Enum with
	Top: EnumInitialDockState
	Bottom: EnumInitialDockState
	Left: EnumInitialDockState
	Right: EnumInitialDockState
	Float: EnumInitialDockState
	function GetEnumItems(self): { EnumInitialDockState }
	function FromName(self, name: string): EnumInitialDockState?
	function FromValue(self, value: number): EnumInitialDockState?
end

declare extern type EnumInputActionType extends EnumItem with
end

declare extern type EnumInputActionType_INTERNAL extends Enum with
	Bool: EnumInputActionType
	Direction1D: EnumInputActionType
	Direction2D: EnumInputActionType
	Direction3D: EnumInputActionType
	ViewportPosition: EnumInputActionType
	function GetEnumItems(self): { EnumInputActionType }
	function FromName(self, name: string): EnumInputActionType?
	function FromValue(self, value: number): EnumInputActionType?
end

declare extern type EnumInputBindingType extends EnumItem with
end

declare extern type EnumInputBindingType_INTERNAL extends Enum with
	Automatic: EnumInputBindingType
	Scriptable: EnumInputBindingType
	function GetEnumItems(self): { EnumInputBindingType }
	function FromName(self, name: string): EnumInputBindingType?
	function FromValue(self, value: number): EnumInputBindingType?
end

declare extern type EnumInputSink extends EnumItem with
end

declare extern type EnumInputSink_INTERNAL extends Enum with
	None: EnumInputSink
	Activate: EnumInputSink
	All: EnumInputSink
	function GetEnumItems(self): { EnumInputSink }
	function FromName(self, name: string): EnumInputSink?
	function FromValue(self, value: number): EnumInputSink?
end

declare extern type EnumInputType extends EnumItem with
end

declare extern type EnumInputType_INTERNAL extends Enum with
	NoInput: EnumInputType
	Constant: EnumInputType
	Sin: EnumInputType
	function GetEnumItems(self): { EnumInputType }
	function FromName(self, name: string): EnumInputType?
	function FromValue(self, value: number): EnumInputType?
end

declare extern type EnumInstanceFileSyncStatus extends EnumItem with
end

declare extern type EnumInstanceFileSyncStatus_INTERNAL extends Enum with
	NotSynced: EnumInstanceFileSyncStatus
	Errored: EnumInstanceFileSyncStatus
	SyncedAsRoot: EnumInstanceFileSyncStatus
	SyncedAsDescendant: EnumInstanceFileSyncStatus
	AncestorErrored: EnumInstanceFileSyncStatus
	function GetEnumItems(self): { EnumInstanceFileSyncStatus }
	function FromName(self, name: string): EnumInstanceFileSyncStatus?
	function FromValue(self, value: number): EnumInstanceFileSyncStatus?
end

declare extern type EnumIntermediateMeshGenerationResult extends EnumItem with
end

declare extern type EnumIntermediateMeshGenerationResult_INTERNAL extends Enum with
	HighQualityMesh: EnumIntermediateMeshGenerationResult
	function GetEnumItems(self): { EnumIntermediateMeshGenerationResult }
	function FromName(self, name: string): EnumIntermediateMeshGenerationResult?
	function FromValue(self, value: number): EnumIntermediateMeshGenerationResult?
end

declare extern type EnumInternalVideoUsage extends EnumItem with
end

declare extern type EnumInternalVideoUsage_INTERNAL extends Enum with
	Default: EnumInternalVideoUsage
	WatchPage: EnumInternalVideoUsage
	HomeCarousel: EnumInternalVideoUsage
	function GetEnumItems(self): { EnumInternalVideoUsage }
	function FromName(self, name: string): EnumInternalVideoUsage?
	function FromValue(self, value: number): EnumInternalVideoUsage?
end

declare extern type EnumInterpolationThrottlingMode extends EnumItem with
end

declare extern type EnumInterpolationThrottlingMode_INTERNAL extends Enum with
	Default: EnumInterpolationThrottlingMode
	Disabled: EnumInterpolationThrottlingMode
	Enabled: EnumInterpolationThrottlingMode
	function GetEnumItems(self): { EnumInterpolationThrottlingMode }
	function FromName(self, name: string): EnumInterpolationThrottlingMode?
	function FromValue(self, value: number): EnumInterpolationThrottlingMode?
end

declare extern type EnumInviteState extends EnumItem with
end

declare extern type EnumInviteState_INTERNAL extends Enum with
	Placed: EnumInviteState
	Accepted: EnumInviteState
	Declined: EnumInviteState
	Missed: EnumInviteState
	function GetEnumItems(self): { EnumInviteState }
	function FromName(self, name: string): EnumInviteState?
	function FromValue(self, value: number): EnumInviteState?
end

declare extern type EnumItemLineAlignment extends EnumItem with
end

declare extern type EnumItemLineAlignment_INTERNAL extends Enum with
	Automatic: EnumItemLineAlignment
	Start: EnumItemLineAlignment
	Center: EnumItemLineAlignment
	End: EnumItemLineAlignment
	Stretch: EnumItemLineAlignment
	function GetEnumItems(self): { EnumItemLineAlignment }
	function FromName(self, name: string): EnumItemLineAlignment?
	function FromValue(self, value: number): EnumItemLineAlignment?
end

declare extern type EnumJoinSource extends EnumItem with
end

declare extern type EnumJoinSource_INTERNAL extends Enum with
	CreatedItemAttribution: EnumJoinSource
	function GetEnumItems(self): { EnumJoinSource }
	function FromName(self, name: string): EnumJoinSource?
	function FromValue(self, value: number): EnumJoinSource?
end

declare extern type EnumJointCreationMode extends EnumItem with
end

declare extern type EnumJointCreationMode_INTERNAL extends Enum with
	All: EnumJointCreationMode
	Surface: EnumJointCreationMode
	None: EnumJointCreationMode
	function GetEnumItems(self): { EnumJointCreationMode }
	function FromName(self, name: string): EnumJointCreationMode?
	function FromValue(self, value: number): EnumJointCreationMode?
end

declare extern type EnumKeyCode extends EnumItem with
end

declare extern type EnumKeyCode_INTERNAL extends Enum with
	Unknown: EnumKeyCode
	Backspace: EnumKeyCode
	Tab: EnumKeyCode
	Clear: EnumKeyCode
	Return: EnumKeyCode
	Pause: EnumKeyCode
	Escape: EnumKeyCode
	Space: EnumKeyCode
	QuotedDouble: EnumKeyCode
	Hash: EnumKeyCode
	Dollar: EnumKeyCode
	Percent: EnumKeyCode
	Ampersand: EnumKeyCode
	Quote: EnumKeyCode
	LeftParenthesis: EnumKeyCode
	RightParenthesis: EnumKeyCode
	Asterisk: EnumKeyCode
	Plus: EnumKeyCode
	Comma: EnumKeyCode
	Minus: EnumKeyCode
	Period: EnumKeyCode
	Slash: EnumKeyCode
	Zero: EnumKeyCode
	One: EnumKeyCode
	Two: EnumKeyCode
	Three: EnumKeyCode
	Four: EnumKeyCode
	Five: EnumKeyCode
	Six: EnumKeyCode
	Seven: EnumKeyCode
	Eight: EnumKeyCode
	Nine: EnumKeyCode
	Colon: EnumKeyCode
	Semicolon: EnumKeyCode
	LessThan: EnumKeyCode
	Equals: EnumKeyCode
	GreaterThan: EnumKeyCode
	Question: EnumKeyCode
	At: EnumKeyCode
	LeftBracket: EnumKeyCode
	BackSlash: EnumKeyCode
	RightBracket: EnumKeyCode
	Caret: EnumKeyCode
	Underscore: EnumKeyCode
	Backquote: EnumKeyCode
	A: EnumKeyCode
	B: EnumKeyCode
	C: EnumKeyCode
	D: EnumKeyCode
	E: EnumKeyCode
	F: EnumKeyCode
	G: EnumKeyCode
	H: EnumKeyCode
	I: EnumKeyCode
	J: EnumKeyCode
	K: EnumKeyCode
	L: EnumKeyCode
	M: EnumKeyCode
	N: EnumKeyCode
	O: EnumKeyCode
	P: EnumKeyCode
	Q: EnumKeyCode
	R: EnumKeyCode
	S: EnumKeyCode
	T: EnumKeyCode
	U: EnumKeyCode
	V: EnumKeyCode
	W: EnumKeyCode
	X: EnumKeyCode
	Y: EnumKeyCode
	Z: EnumKeyCode
	LeftCurly: EnumKeyCode
	Pipe: EnumKeyCode
	RightCurly: EnumKeyCode
	Tilde: EnumKeyCode
	Delete: EnumKeyCode
	World0: EnumKeyCode
	World1: EnumKeyCode
	World2: EnumKeyCode
	World3: EnumKeyCode
	World4: EnumKeyCode
	World5: EnumKeyCode
	World6: EnumKeyCode
	World7: EnumKeyCode
	World8: EnumKeyCode
	World9: EnumKeyCode
	World10: EnumKeyCode
	World11: EnumKeyCode
	World12: EnumKeyCode
	World13: EnumKeyCode
	World14: EnumKeyCode
	World15: EnumKeyCode
	World16: EnumKeyCode
	World17: EnumKeyCode
	World18: EnumKeyCode
	World19: EnumKeyCode
	World20: EnumKeyCode
	World21: EnumKeyCode
	World22: EnumKeyCode
	World23: EnumKeyCode
	World24: EnumKeyCode
	World25: EnumKeyCode
	World26: EnumKeyCode
	World27: EnumKeyCode
	World28: EnumKeyCode
	World29: EnumKeyCode
	World30: EnumKeyCode
	World31: EnumKeyCode
	World32: EnumKeyCode
	World33: EnumKeyCode
	World34: EnumKeyCode
	World35: EnumKeyCode
	World36: EnumKeyCode
	World37: EnumKeyCode
	World38: EnumKeyCode
	World39: EnumKeyCode
	World40: EnumKeyCode
	World41: EnumKeyCode
	World42: EnumKeyCode
	World43: EnumKeyCode
	World44: EnumKeyCode
	World45: EnumKeyCode
	World46: EnumKeyCode
	World47: EnumKeyCode
	World48: EnumKeyCode
	World49: EnumKeyCode
	World50: EnumKeyCode
	World51: EnumKeyCode
	World52: EnumKeyCode
	World53: EnumKeyCode
	World54: EnumKeyCode
	World55: EnumKeyCode
	World56: EnumKeyCode
	World57: EnumKeyCode
	World58: EnumKeyCode
	World59: EnumKeyCode
	World60: EnumKeyCode
	World61: EnumKeyCode
	World62: EnumKeyCode
	World63: EnumKeyCode
	World64: EnumKeyCode
	World65: EnumKeyCode
	World66: EnumKeyCode
	World67: EnumKeyCode
	World68: EnumKeyCode
	World69: EnumKeyCode
	World70: EnumKeyCode
	World71: EnumKeyCode
	World72: EnumKeyCode
	World73: EnumKeyCode
	World74: EnumKeyCode
	World75: EnumKeyCode
	World76: EnumKeyCode
	World77: EnumKeyCode
	World78: EnumKeyCode
	World79: EnumKeyCode
	World80: EnumKeyCode
	World81: EnumKeyCode
	World82: EnumKeyCode
	World83: EnumKeyCode
	World84: EnumKeyCode
	World85: EnumKeyCode
	World86: EnumKeyCode
	World87: EnumKeyCode
	World88: EnumKeyCode
	World89: EnumKeyCode
	World90: EnumKeyCode
	World91: EnumKeyCode
	World92: EnumKeyCode
	World93: EnumKeyCode
	World94: EnumKeyCode
	World95: EnumKeyCode
	KeypadZero: EnumKeyCode
	KeypadOne: EnumKeyCode
	KeypadTwo: EnumKeyCode
	KeypadThree: EnumKeyCode
	KeypadFour: EnumKeyCode
	KeypadFive: EnumKeyCode
	KeypadSix: EnumKeyCode
	KeypadSeven: EnumKeyCode
	KeypadEight: EnumKeyCode
	KeypadNine: EnumKeyCode
	KeypadPeriod: EnumKeyCode
	KeypadDivide: EnumKeyCode
	KeypadMultiply: EnumKeyCode
	KeypadMinus: EnumKeyCode
	KeypadPlus: EnumKeyCode
	KeypadEnter: EnumKeyCode
	KeypadEquals: EnumKeyCode
	Up: EnumKeyCode
	Down: EnumKeyCode
	Right: EnumKeyCode
	Left: EnumKeyCode
	Insert: EnumKeyCode
	Home: EnumKeyCode
	End: EnumKeyCode
	PageUp: EnumKeyCode
	PageDown: EnumKeyCode
	F1: EnumKeyCode
	F2: EnumKeyCode
	F3: EnumKeyCode
	F4: EnumKeyCode
	F5: EnumKeyCode
	F6: EnumKeyCode
	F7: EnumKeyCode
	F8: EnumKeyCode
	F9: EnumKeyCode
	F10: EnumKeyCode
	F11: EnumKeyCode
	F12: EnumKeyCode
	F13: EnumKeyCode
	F14: EnumKeyCode
	F15: EnumKeyCode
	NumLock: EnumKeyCode
	CapsLock: EnumKeyCode
	ScrollLock: EnumKeyCode
	RightShift: EnumKeyCode
	LeftShift: EnumKeyCode
	RightControl: EnumKeyCode
	LeftControl: EnumKeyCode
	RightAlt: EnumKeyCode
	LeftAlt: EnumKeyCode
	RightMeta: EnumKeyCode
	LeftMeta: EnumKeyCode
	LeftSuper: EnumKeyCode
	RightSuper: EnumKeyCode
	Mode: EnumKeyCode
	Compose: EnumKeyCode
	Help: EnumKeyCode
	Print: EnumKeyCode
	SysReq: EnumKeyCode
	Break: EnumKeyCode
	Menu: EnumKeyCode
	Power: EnumKeyCode
	Euro: EnumKeyCode
	Undo: EnumKeyCode
	ButtonX: EnumKeyCode
	ButtonY: EnumKeyCode
	ButtonA: EnumKeyCode
	ButtonB: EnumKeyCode
	ButtonR1: EnumKeyCode
	ButtonL1: EnumKeyCode
	ButtonR2: EnumKeyCode
	ButtonL2: EnumKeyCode
	ButtonR3: EnumKeyCode
	ButtonL3: EnumKeyCode
	ButtonStart: EnumKeyCode
	ButtonSelect: EnumKeyCode
	DPadLeft: EnumKeyCode
	DPadRight: EnumKeyCode
	DPadUp: EnumKeyCode
	DPadDown: EnumKeyCode
	Thumbstick1: EnumKeyCode
	Thumbstick2: EnumKeyCode
	Thumbstick1Up: EnumKeyCode
	Thumbstick1Down: EnumKeyCode
	Thumbstick1Left: EnumKeyCode
	Thumbstick1Right: EnumKeyCode
	Thumbstick2Up: EnumKeyCode
	Thumbstick2Down: EnumKeyCode
	Thumbstick2Left: EnumKeyCode
	Thumbstick2Right: EnumKeyCode
	MouseLeftButton: EnumKeyCode
	MouseRightButton: EnumKeyCode
	MouseMiddleButton: EnumKeyCode
	MouseBackButton: EnumKeyCode
	MouseNoButton: EnumKeyCode
	MouseX: EnumKeyCode
	MouseY: EnumKeyCode
	MousePosition: EnumKeyCode
	TouchPosition: EnumKeyCode
	MouseWheel: EnumKeyCode
	TrackpadPan: EnumKeyCode
	TrackpadPinch: EnumKeyCode
	MouseDelta: EnumKeyCode
	TouchDelta: EnumKeyCode
	TouchPinch: EnumKeyCode
	function GetEnumItems(self): { EnumKeyCode }
	function FromName(self, name: string): EnumKeyCode?
	function FromValue(self, value: number): EnumKeyCode?
end

declare extern type EnumKeyCodeStringFormat extends EnumItem with
end

declare extern type EnumKeyCodeStringFormat_INTERNAL extends Enum with
	Default: EnumKeyCodeStringFormat
	Abbreviated: EnumKeyCodeStringFormat
	function GetEnumItems(self): { EnumKeyCodeStringFormat }
	function FromName(self, name: string): EnumKeyCodeStringFormat?
	function FromValue(self, value: number): EnumKeyCodeStringFormat?
end

declare extern type EnumKeyInterpolationMode extends EnumItem with
end

declare extern type EnumKeyInterpolationMode_INTERNAL extends Enum with
	Constant: EnumKeyInterpolationMode
	Linear: EnumKeyInterpolationMode
	Cubic: EnumKeyInterpolationMode
	function GetEnumItems(self): { EnumKeyInterpolationMode }
	function FromName(self, name: string): EnumKeyInterpolationMode?
	function FromValue(self, value: number): EnumKeyInterpolationMode?
end

declare extern type EnumKeywordFilterType extends EnumItem with
end

declare extern type EnumKeywordFilterType_INTERNAL extends Enum with
	Include: EnumKeywordFilterType
	Exclude: EnumKeywordFilterType
	function GetEnumItems(self): { EnumKeywordFilterType }
	function FromName(self, name: string): EnumKeywordFilterType?
	function FromValue(self, value: number): EnumKeywordFilterType?
end

declare extern type EnumLanguage extends EnumItem with
end

declare extern type EnumLanguage_INTERNAL extends Enum with
	Default: EnumLanguage
	function GetEnumItems(self): { EnumLanguage }
	function FromName(self, name: string): EnumLanguage?
	function FromValue(self, value: number): EnumLanguage?
end

declare extern type EnumLeftRight extends EnumItem with
end

declare extern type EnumLeftRight_INTERNAL extends Enum with
	Left: EnumLeftRight
	Center: EnumLeftRight
	Right: EnumLeftRight
	function GetEnumItems(self): { EnumLeftRight }
	function FromName(self, name: string): EnumLeftRight?
	function FromValue(self, value: number): EnumLeftRight?
end

declare extern type EnumLexemeType extends EnumItem with
end

declare extern type EnumLexemeType_INTERNAL extends Enum with
	Eof: EnumLexemeType
	Name: EnumLexemeType
	QuotedString: EnumLexemeType
	Number: EnumLexemeType
	And: EnumLexemeType
	Or: EnumLexemeType
	Equal: EnumLexemeType
	TildeEqual: EnumLexemeType
	GreaterThan: EnumLexemeType
	GreaterThanEqual: EnumLexemeType
	LessThan: EnumLexemeType
	LessThanEqual: EnumLexemeType
	Colon: EnumLexemeType
	Dot: EnumLexemeType
	LeftParenthesis: EnumLexemeType
	RightParenthesis: EnumLexemeType
	Star: EnumLexemeType
	DoubleStar: EnumLexemeType
	ReservedSpecial: EnumLexemeType
	function GetEnumItems(self): { EnumLexemeType }
	function FromName(self, name: string): EnumLexemeType?
	function FromValue(self, value: number): EnumLexemeType?
end

declare extern type EnumLightingStyle extends EnumItem with
end

declare extern type EnumLightingStyle_INTERNAL extends Enum with
	Realistic: EnumLightingStyle
	Soft: EnumLightingStyle
	function GetEnumItems(self): { EnumLightingStyle }
	function FromName(self, name: string): EnumLightingStyle?
	function FromValue(self, value: number): EnumLightingStyle?
end

declare extern type EnumLimb extends EnumItem with
end

declare extern type EnumLimb_INTERNAL extends Enum with
	Head: EnumLimb
	Torso: EnumLimb
	LeftArm: EnumLimb
	RightArm: EnumLimb
	LeftLeg: EnumLimb
	RightLeg: EnumLimb
	Unknown: EnumLimb
	function GetEnumItems(self): { EnumLimb }
	function FromName(self, name: string): EnumLimb?
	function FromValue(self, value: number): EnumLimb?
end

declare extern type EnumLineJoinMode extends EnumItem with
end

declare extern type EnumLineJoinMode_INTERNAL extends Enum with
	Round: EnumLineJoinMode
	Bevel: EnumLineJoinMode
	Miter: EnumLineJoinMode
	function GetEnumItems(self): { EnumLineJoinMode }
	function FromName(self, name: string): EnumLineJoinMode?
	function FromValue(self, value: number): EnumLineJoinMode?
end

declare extern type EnumListDisplayMode extends EnumItem with
end

declare extern type EnumListDisplayMode_INTERNAL extends Enum with
	Horizontal: EnumListDisplayMode
	Vertical: EnumListDisplayMode
	function GetEnumItems(self): { EnumListDisplayMode }
	function FromName(self, name: string): EnumListDisplayMode?
	function FromValue(self, value: number): EnumListDisplayMode?
end

declare extern type EnumListenerLocation extends EnumItem with
end

declare extern type EnumListenerLocation_INTERNAL extends Enum with
	Default: EnumListenerLocation
	None: EnumListenerLocation
	Character: EnumListenerLocation
	Camera: EnumListenerLocation
	function GetEnumItems(self): { EnumListenerLocation }
	function FromName(self, name: string): EnumListenerLocation?
	function FromValue(self, value: number): EnumListenerLocation?
end

declare extern type EnumListenerPositionType extends EnumItem with
end

declare extern type EnumListenerPositionType_INTERNAL extends Enum with
	Parent: EnumListenerPositionType
	Instance: EnumListenerPositionType
	function GetEnumItems(self): { EnumListenerPositionType }
	function FromName(self, name: string): EnumListenerPositionType?
	function FromValue(self, value: number): EnumListenerPositionType?
end

declare extern type EnumListenerType extends EnumItem with
end

declare extern type EnumListenerType_INTERNAL extends Enum with
	Camera: EnumListenerType
	CFrame: EnumListenerType
	ObjectPosition: EnumListenerType
	ObjectCFrame: EnumListenerType
	function GetEnumItems(self): { EnumListenerType }
	function FromName(self, name: string): EnumListenerType?
	function FromValue(self, value: number): EnumListenerType?
end

declare extern type EnumLiveEditingAtomicUpdateResponse extends EnumItem with
end

declare extern type EnumLiveEditingAtomicUpdateResponse_INTERNAL extends Enum with
	Success: EnumLiveEditingAtomicUpdateResponse
	FailureGuidNotFound: EnumLiveEditingAtomicUpdateResponse
	FailureHashMismatch: EnumLiveEditingAtomicUpdateResponse
	FailureOperationIllegal: EnumLiveEditingAtomicUpdateResponse
	function GetEnumItems(self): { EnumLiveEditingAtomicUpdateResponse }
	function FromName(self, name: string): EnumLiveEditingAtomicUpdateResponse?
	function FromValue(self, value: number): EnumLiveEditingAtomicUpdateResponse?
end

declare extern type EnumLiveEditingBroadcastMessageType extends EnumItem with
end

declare extern type EnumLiveEditingBroadcastMessageType_INTERNAL extends Enum with
	Normal: EnumLiveEditingBroadcastMessageType
	Warning: EnumLiveEditingBroadcastMessageType
	Error: EnumLiveEditingBroadcastMessageType
	function GetEnumItems(self): { EnumLiveEditingBroadcastMessageType }
	function FromName(self, name: string): EnumLiveEditingBroadcastMessageType?
	function FromValue(self, value: number): EnumLiveEditingBroadcastMessageType?
end

declare extern type EnumLoadCharacterLayeredClothing extends EnumItem with
end

declare extern type EnumLoadCharacterLayeredClothing_INTERNAL extends Enum with
	Default: EnumLoadCharacterLayeredClothing
	Disabled: EnumLoadCharacterLayeredClothing
	Enabled: EnumLoadCharacterLayeredClothing
	function GetEnumItems(self): { EnumLoadCharacterLayeredClothing }
	function FromName(self, name: string): EnumLoadCharacterLayeredClothing?
	function FromValue(self, value: number): EnumLoadCharacterLayeredClothing?
end

declare extern type EnumLoadDynamicHeads extends EnumItem with
end

declare extern type EnumLoadDynamicHeads_INTERNAL extends Enum with
	Default: EnumLoadDynamicHeads
	Disabled: EnumLoadDynamicHeads
	Enabled: EnumLoadDynamicHeads
	function GetEnumItems(self): { EnumLoadDynamicHeads }
	function FromName(self, name: string): EnumLoadDynamicHeads?
	function FromValue(self, value: number): EnumLoadDynamicHeads?
end

declare extern type EnumLocationType extends EnumItem with
end

declare extern type EnumLocationType_INTERNAL extends Enum with
	Character: EnumLocationType
	Camera: EnumLocationType
	ObjectPosition: EnumLocationType
	function GetEnumItems(self): { EnumLocationType }
	function FromName(self, name: string): EnumLocationType?
	function FromValue(self, value: number): EnumLocationType?
end

declare extern type EnumLuauTypeCheckMode extends EnumItem with
end

declare extern type EnumLuauTypeCheckMode_INTERNAL extends Enum with
	Default: EnumLuauTypeCheckMode
	NoCheck: EnumLuauTypeCheckMode
	Nonstrict: EnumLuauTypeCheckMode
	Strict: EnumLuauTypeCheckMode
	function GetEnumItems(self): { EnumLuauTypeCheckMode }
	function FromName(self, name: string): EnumLuauTypeCheckMode?
	function FromValue(self, value: number): EnumLuauTypeCheckMode?
end

declare extern type EnumMakeupType extends EnumItem with
end

declare extern type EnumMakeupType_INTERNAL extends Enum with
	Face: EnumMakeupType
	Lip: EnumMakeupType
	Eye: EnumMakeupType
	function GetEnumItems(self): { EnumMakeupType }
	function FromName(self, name: string): EnumMakeupType?
	function FromValue(self, value: number): EnumMakeupType?
end

declare extern type EnumMarketplaceBulkPurchasePromptStatus extends EnumItem with
end

declare extern type EnumMarketplaceBulkPurchasePromptStatus_INTERNAL extends Enum with
	Completed: EnumMarketplaceBulkPurchasePromptStatus
	Aborted: EnumMarketplaceBulkPurchasePromptStatus
	Error: EnumMarketplaceBulkPurchasePromptStatus
	function GetEnumItems(self): { EnumMarketplaceBulkPurchasePromptStatus }
	function FromName(self, name: string): EnumMarketplaceBulkPurchasePromptStatus?
	function FromValue(self, value: number): EnumMarketplaceBulkPurchasePromptStatus?
end

declare extern type EnumMarketplaceItemPurchaseStatus extends EnumItem with
end

declare extern type EnumMarketplaceItemPurchaseStatus_INTERNAL extends Enum with
	Success: EnumMarketplaceItemPurchaseStatus
	SystemError: EnumMarketplaceItemPurchaseStatus
	AlreadyOwned: EnumMarketplaceItemPurchaseStatus
	InsufficientRobux: EnumMarketplaceItemPurchaseStatus
	QuantityLimitExceeded: EnumMarketplaceItemPurchaseStatus
	QuotaExceeded: EnumMarketplaceItemPurchaseStatus
	NotForSale: EnumMarketplaceItemPurchaseStatus
	NotAvailableForPurchaser: EnumMarketplaceItemPurchaseStatus
	PriceMismatch: EnumMarketplaceItemPurchaseStatus
	SoldOut: EnumMarketplaceItemPurchaseStatus
	PurchaserIsSeller: EnumMarketplaceItemPurchaseStatus
	InsufficientMembership: EnumMarketplaceItemPurchaseStatus
	PlaceInvalid: EnumMarketplaceItemPurchaseStatus
	function GetEnumItems(self): { EnumMarketplaceItemPurchaseStatus }
	function FromName(self, name: string): EnumMarketplaceItemPurchaseStatus?
	function FromValue(self, value: number): EnumMarketplaceItemPurchaseStatus?
end

declare extern type EnumMarketplaceProductType extends EnumItem with
end

declare extern type EnumMarketplaceProductType_INTERNAL extends Enum with
	AvatarAsset: EnumMarketplaceProductType
	AvatarBundle: EnumMarketplaceProductType
	function GetEnumItems(self): { EnumMarketplaceProductType }
	function FromName(self, name: string): EnumMarketplaceProductType?
	function FromValue(self, value: number): EnumMarketplaceProductType?
end

declare extern type EnumMarkupKind extends EnumItem with
end

declare extern type EnumMarkupKind_INTERNAL extends Enum with
	PlainText: EnumMarkupKind
	Markdown: EnumMarkupKind
	function GetEnumItems(self): { EnumMarkupKind }
	function FromName(self, name: string): EnumMarkupKind?
	function FromValue(self, value: number): EnumMarkupKind?
end

declare extern type EnumMatchmakingType extends EnumItem with
end

declare extern type EnumMatchmakingType_INTERNAL extends Enum with
	Default: EnumMatchmakingType
	XboxOnly: EnumMatchmakingType
	PlayStationOnly: EnumMatchmakingType
	function GetEnumItems(self): { EnumMatchmakingType }
	function FromName(self, name: string): EnumMatchmakingType?
	function FromValue(self, value: number): EnumMatchmakingType?
end

declare extern type EnumMaterial extends EnumItem with
end

declare extern type EnumMaterial_INTERNAL extends Enum with
	Plastic: EnumMaterial
	SmoothPlastic: EnumMaterial
	Neon: EnumMaterial
	Wood: EnumMaterial
	WoodPlanks: EnumMaterial
	Marble: EnumMaterial
	Basalt: EnumMaterial
	Slate: EnumMaterial
	CrackedLava: EnumMaterial
	Concrete: EnumMaterial
	Limestone: EnumMaterial
	Granite: EnumMaterial
	Pavement: EnumMaterial
	Brick: EnumMaterial
	Pebble: EnumMaterial
	Cobblestone: EnumMaterial
	Rock: EnumMaterial
	Sandstone: EnumMaterial
	CorrodedMetal: EnumMaterial
	DiamondPlate: EnumMaterial
	Foil: EnumMaterial
	Metal: EnumMaterial
	Grass: EnumMaterial
	LeafyGrass: EnumMaterial
	Sand: EnumMaterial
	Fabric: EnumMaterial
	Snow: EnumMaterial
	Mud: EnumMaterial
	Ground: EnumMaterial
	Asphalt: EnumMaterial
	Salt: EnumMaterial
	Ice: EnumMaterial
	Glacier: EnumMaterial
	Glass: EnumMaterial
	ForceField: EnumMaterial
	Air: EnumMaterial
	Water: EnumMaterial
	Cardboard: EnumMaterial
	Carpet: EnumMaterial
	CeramicTiles: EnumMaterial
	ClayRoofTiles: EnumMaterial
	RoofShingles: EnumMaterial
	Leather: EnumMaterial
	Plaster: EnumMaterial
	Rubber: EnumMaterial
	function GetEnumItems(self): { EnumMaterial }
	function FromName(self, name: string): EnumMaterial?
	function FromValue(self, value: number): EnumMaterial?
end

declare extern type EnumMaterialPattern extends EnumItem with
end

declare extern type EnumMaterialPattern_INTERNAL extends Enum with
	Regular: EnumMaterialPattern
	Organic: EnumMaterialPattern
	function GetEnumItems(self): { EnumMaterialPattern }
	function FromName(self, name: string): EnumMaterialPattern?
	function FromValue(self, value: number): EnumMaterialPattern?
end

declare extern type EnumMembershipType extends EnumItem with
end

declare extern type EnumMembershipType_INTERNAL extends Enum with
	None: EnumMembershipType
	BuildersClub: EnumMembershipType
	TurboBuildersClub: EnumMembershipType
	OutrageousBuildersClub: EnumMembershipType
	Premium: EnumMembershipType
	function GetEnumItems(self): { EnumMembershipType }
	function FromName(self, name: string): EnumMembershipType?
	function FromValue(self, value: number): EnumMembershipType?
end

declare extern type EnumMeshPartDetailLevel extends EnumItem with
end

declare extern type EnumMeshPartDetailLevel_INTERNAL extends Enum with
	DistanceBased: EnumMeshPartDetailLevel
	Level00: EnumMeshPartDetailLevel
	Level01: EnumMeshPartDetailLevel
	Level02: EnumMeshPartDetailLevel
	Level03: EnumMeshPartDetailLevel
	Level04: EnumMeshPartDetailLevel
	Level05: EnumMeshPartDetailLevel
	Level06: EnumMeshPartDetailLevel
	Level07: EnumMeshPartDetailLevel
	Level08: EnumMeshPartDetailLevel
	Level09: EnumMeshPartDetailLevel
	function GetEnumItems(self): { EnumMeshPartDetailLevel }
	function FromName(self, name: string): EnumMeshPartDetailLevel?
	function FromValue(self, value: number): EnumMeshPartDetailLevel?
end

declare extern type EnumMeshPartHeadsAndAccessories extends EnumItem with
end

declare extern type EnumMeshPartHeadsAndAccessories_INTERNAL extends Enum with
	Default: EnumMeshPartHeadsAndAccessories
	Disabled: EnumMeshPartHeadsAndAccessories
	Enabled: EnumMeshPartHeadsAndAccessories
	function GetEnumItems(self): { EnumMeshPartHeadsAndAccessories }
	function FromName(self, name: string): EnumMeshPartHeadsAndAccessories?
	function FromValue(self, value: number): EnumMeshPartHeadsAndAccessories?
end

declare extern type EnumMeshScaleUnit extends EnumItem with
end

declare extern type EnumMeshScaleUnit_INTERNAL extends Enum with
	Stud: EnumMeshScaleUnit
	Meter: EnumMeshScaleUnit
	CM: EnumMeshScaleUnit
	MM: EnumMeshScaleUnit
	Foot: EnumMeshScaleUnit
	Inch: EnumMeshScaleUnit
	function GetEnumItems(self): { EnumMeshScaleUnit }
	function FromName(self, name: string): EnumMeshScaleUnit?
	function FromValue(self, value: number): EnumMeshScaleUnit?
end

declare extern type EnumMeshType extends EnumItem with
end

declare extern type EnumMeshType_INTERNAL extends Enum with
	Head: EnumMeshType
	Torso: EnumMeshType
	Wedge: EnumMeshType
	Sphere: EnumMeshType
	Cylinder: EnumMeshType
	FileMesh: EnumMeshType
	Brick: EnumMeshType
	Prism: EnumMeshType
	Pyramid: EnumMeshType
	ParallelRamp: EnumMeshType
	RightAngleRamp: EnumMeshType
	CornerWedge: EnumMeshType
	function GetEnumItems(self): { EnumMeshType }
	function FromName(self, name: string): EnumMeshType?
	function FromValue(self, value: number): EnumMeshType?
end

declare extern type EnumMessageType extends EnumItem with
end

declare extern type EnumMessageType_INTERNAL extends Enum with
	MessageOutput: EnumMessageType
	MessageInfo: EnumMessageType
	MessageWarning: EnumMessageType
	MessageError: EnumMessageType
	function GetEnumItems(self): { EnumMessageType }
	function FromName(self, name: string): EnumMessageType?
	function FromValue(self, value: number): EnumMessageType?
end

declare extern type EnumModelLevelOfDetail extends EnumItem with
end

declare extern type EnumModelLevelOfDetail_INTERNAL extends Enum with
	Automatic: EnumModelLevelOfDetail
	StreamingMesh: EnumModelLevelOfDetail
	Disabled: EnumModelLevelOfDetail
	SLIM: EnumModelLevelOfDetail
	function GetEnumItems(self): { EnumModelLevelOfDetail }
	function FromName(self, name: string): EnumModelLevelOfDetail?
	function FromValue(self, value: number): EnumModelLevelOfDetail?
end

declare extern type EnumModelStreamingBehavior extends EnumItem with
end

declare extern type EnumModelStreamingBehavior_INTERNAL extends Enum with
	Default: EnumModelStreamingBehavior
	Legacy: EnumModelStreamingBehavior
	Improved: EnumModelStreamingBehavior
	function GetEnumItems(self): { EnumModelStreamingBehavior }
	function FromName(self, name: string): EnumModelStreamingBehavior?
	function FromValue(self, value: number): EnumModelStreamingBehavior?
end

declare extern type EnumModelStreamingMode extends EnumItem with
end

declare extern type EnumModelStreamingMode_INTERNAL extends Enum with
	Default: EnumModelStreamingMode
	Atomic: EnumModelStreamingMode
	Persistent: EnumModelStreamingMode
	PersistentPerPlayer: EnumModelStreamingMode
	Nonatomic: EnumModelStreamingMode
	function GetEnumItems(self): { EnumModelStreamingMode }
	function FromName(self, name: string): EnumModelStreamingMode?
	function FromValue(self, value: number): EnumModelStreamingMode?
end

declare extern type EnumModerationResultCategory extends EnumItem with
end

declare extern type EnumModerationResultCategory_INTERNAL extends Enum with
	ViolationDetected: EnumModerationResultCategory
	Borderline: EnumModerationResultCategory
	NoViolationDetected: EnumModerationResultCategory
	function GetEnumItems(self): { EnumModerationResultCategory }
	function FromName(self, name: string): EnumModerationResultCategory?
	function FromValue(self, value: number): EnumModerationResultCategory?
end

declare extern type EnumModerationResultLabel extends EnumItem with
end

declare extern type EnumModerationResultLabel_INTERNAL extends Enum with
	ChildExploitation: EnumModerationResultLabel
	SuicideSelfInjuryAndHarmfulBehavior: EnumModerationResultLabel
	ThreatsBullyingAndHarassment: EnumModerationResultLabel
	TerrorismAndViolentExtremism: EnumModerationResultLabel
	DiscriminationSlursAndHateSpeech: EnumModerationResultLabel
	RealWorldSensitiveEvents: EnumModerationResultLabel
	ViolentContentAndGore: EnumModerationResultLabel
	RomanticAndSexualContent: EnumModerationResultLabel
	IllegalAndRegulatedGoodsAndActivities: EnumModerationResultLabel
	Profanity: EnumModerationResultLabel
	Other: EnumModerationResultLabel
	function GetEnumItems(self): { EnumModerationResultLabel }
	function FromName(self, name: string): EnumModerationResultLabel?
	function FromValue(self, value: number): EnumModerationResultLabel?
end

declare extern type EnumModerationStatus extends EnumItem with
end

declare extern type EnumModerationStatus_INTERNAL extends Enum with
	ReviewedApproved: EnumModerationStatus
	ReviewedRejected: EnumModerationStatus
	NotReviewed: EnumModerationStatus
	NotApplicable: EnumModerationStatus
	Invalid: EnumModerationStatus
	function GetEnumItems(self): { EnumModerationStatus }
	function FromName(self, name: string): EnumModerationStatus?
	function FromValue(self, value: number): EnumModerationStatus?
end

declare extern type EnumModifierKey extends EnumItem with
end

declare extern type EnumModifierKey_INTERNAL extends Enum with
	Shift: EnumModifierKey
	Ctrl: EnumModifierKey
	Alt: EnumModifierKey
	Meta: EnumModifierKey
	function GetEnumItems(self): { EnumModifierKey }
	function FromName(self, name: string): EnumModifierKey?
	function FromValue(self, value: number): EnumModifierKey?
end

declare extern type EnumMouseBehavior extends EnumItem with
end

declare extern type EnumMouseBehavior_INTERNAL extends Enum with
	Default: EnumMouseBehavior
	LockCenter: EnumMouseBehavior
	LockCurrentPosition: EnumMouseBehavior
	function GetEnumItems(self): { EnumMouseBehavior }
	function FromName(self, name: string): EnumMouseBehavior?
	function FromValue(self, value: number): EnumMouseBehavior?
end

declare extern type EnumMoveState extends EnumItem with
end

declare extern type EnumMoveState_INTERNAL extends Enum with
	Stopped: EnumMoveState
	Coasting: EnumMoveState
	Pushing: EnumMoveState
	Stopping: EnumMoveState
	AirFree: EnumMoveState
	function GetEnumItems(self): { EnumMoveState }
	function FromName(self, name: string): EnumMoveState?
	function FromValue(self, value: number): EnumMoveState?
end

declare extern type EnumMuteState extends EnumItem with
end

declare extern type EnumMuteState_INTERNAL extends Enum with
	Unmuted: EnumMuteState
	Muted: EnumMuteState
	function GetEnumItems(self): { EnumMuteState }
	function FromName(self, name: string): EnumMuteState?
	function FromValue(self, value: number): EnumMuteState?
end

declare extern type EnumNameOcclusion extends EnumItem with
end

declare extern type EnumNameOcclusion_INTERNAL extends Enum with
	NoOcclusion: EnumNameOcclusion
	EnemyOcclusion: EnumNameOcclusion
	OccludeAll: EnumNameOcclusion
	function GetEnumItems(self): { EnumNameOcclusion }
	function FromName(self, name: string): EnumNameOcclusion?
	function FromValue(self, value: number): EnumNameOcclusion?
end

declare extern type EnumNegateOperationHiddenHistory extends EnumItem with
end

declare extern type EnumNegateOperationHiddenHistory_INTERNAL extends Enum with
	None: EnumNegateOperationHiddenHistory
	NegatedUnion: EnumNegateOperationHiddenHistory
	NegatedIntersection: EnumNegateOperationHiddenHistory
	function GetEnumItems(self): { EnumNegateOperationHiddenHistory }
	function FromName(self, name: string): EnumNegateOperationHiddenHistory?
	function FromValue(self, value: number): EnumNegateOperationHiddenHistory?
end

declare extern type EnumNetworkOwnership extends EnumItem with
end

declare extern type EnumNetworkOwnership_INTERNAL extends Enum with
	Automatic: EnumNetworkOwnership
	Manual: EnumNetworkOwnership
	OnContact: EnumNetworkOwnership
	function GetEnumItems(self): { EnumNetworkOwnership }
	function FromName(self, name: string): EnumNetworkOwnership?
	function FromValue(self, value: number): EnumNetworkOwnership?
end

declare extern type EnumNetworkStatus extends EnumItem with
end

declare extern type EnumNetworkStatus_INTERNAL extends Enum with
	Unknown: EnumNetworkStatus
	Connected: EnumNetworkStatus
	Disconnected: EnumNetworkStatus
	function GetEnumItems(self): { EnumNetworkStatus }
	function FromName(self, name: string): EnumNetworkStatus?
	function FromValue(self, value: number): EnumNetworkStatus?
end

declare extern type EnumNoiseType extends EnumItem with
end

declare extern type EnumNoiseType_INTERNAL extends Enum with
	SimplexGabor: EnumNoiseType
	function GetEnumItems(self): { EnumNoiseType }
	function FromName(self, name: string): EnumNoiseType?
	function FromValue(self, value: number): EnumNoiseType?
end

declare extern type EnumNormalId extends EnumItem with
end

declare extern type EnumNormalId_INTERNAL extends Enum with
	Right: EnumNormalId
	Top: EnumNormalId
	Back: EnumNormalId
	Left: EnumNormalId
	Bottom: EnumNormalId
	Front: EnumNormalId
	function GetEnumItems(self): { EnumNormalId }
	function FromName(self, name: string): EnumNormalId?
	function FromValue(self, value: number): EnumNormalId?
end

declare extern type EnumNotificationButtonType extends EnumItem with
end

declare extern type EnumNotificationButtonType_INTERNAL extends Enum with
	Primary: EnumNotificationButtonType
	Secondary: EnumNotificationButtonType
	function GetEnumItems(self): { EnumNotificationButtonType }
	function FromName(self, name: string): EnumNotificationButtonType?
	function FromValue(self, value: number): EnumNotificationButtonType?
end

declare extern type EnumOperationType extends EnumItem with
end

declare extern type EnumOperationType_INTERNAL extends Enum with
	Null: EnumOperationType
	Union: EnumOperationType
	Subtraction: EnumOperationType
	Intersection: EnumOperationType
	Primitive: EnumOperationType
	function GetEnumItems(self): { EnumOperationType }
	function FromName(self, name: string): EnumOperationType?
	function FromValue(self, value: number): EnumOperationType?
end

declare extern type EnumOrientationAlignmentMode extends EnumItem with
end

declare extern type EnumOrientationAlignmentMode_INTERNAL extends Enum with
	OneAttachment: EnumOrientationAlignmentMode
	TwoAttachment: EnumOrientationAlignmentMode
	function GetEnumItems(self): { EnumOrientationAlignmentMode }
	function FromName(self, name: string): EnumOrientationAlignmentMode?
	function FromValue(self, value: number): EnumOrientationAlignmentMode?
end

declare extern type EnumOutfitSource extends EnumItem with
end

declare extern type EnumOutfitSource_INTERNAL extends Enum with
	All: EnumOutfitSource
	Created: EnumOutfitSource
	Purchased: EnumOutfitSource
	function GetEnumItems(self): { EnumOutfitSource }
	function FromName(self, name: string): EnumOutfitSource?
	function FromValue(self, value: number): EnumOutfitSource?
end

declare extern type EnumOutfitType extends EnumItem with
end

declare extern type EnumOutfitType_INTERNAL extends Enum with
	All: EnumOutfitType
	Avatar: EnumOutfitType
	DynamicHead: EnumOutfitType
	Shoes: EnumOutfitType
	Makeup: EnumOutfitType
	function GetEnumItems(self): { EnumOutfitType }
	function FromName(self, name: string): EnumOutfitType?
	function FromValue(self, value: number): EnumOutfitType?
end

declare extern type EnumOutputLayoutMode extends EnumItem with
end

declare extern type EnumOutputLayoutMode_INTERNAL extends Enum with
	Horizontal: EnumOutputLayoutMode
	Vertical: EnumOutputLayoutMode
	function GetEnumItems(self): { EnumOutputLayoutMode }
	function FromName(self, name: string): EnumOutputLayoutMode?
	function FromValue(self, value: number): EnumOutputLayoutMode?
end

declare extern type EnumOverrideMouseIconBehavior extends EnumItem with
end

declare extern type EnumOverrideMouseIconBehavior_INTERNAL extends Enum with
	None: EnumOverrideMouseIconBehavior
	ForceShow: EnumOverrideMouseIconBehavior
	ForceHide: EnumOverrideMouseIconBehavior
	function GetEnumItems(self): { EnumOverrideMouseIconBehavior }
	function FromName(self, name: string): EnumOverrideMouseIconBehavior?
	function FromValue(self, value: number): EnumOverrideMouseIconBehavior?
end

declare extern type EnumPackagePermission extends EnumItem with
end

declare extern type EnumPackagePermission_INTERNAL extends Enum with
	None: EnumPackagePermission
	NoAccess: EnumPackagePermission
	Revoked: EnumPackagePermission
	UseView: EnumPackagePermission
	Edit: EnumPackagePermission
	Own: EnumPackagePermission
	function GetEnumItems(self): { EnumPackagePermission }
	function FromName(self, name: string): EnumPackagePermission?
	function FromValue(self, value: number): EnumPackagePermission?
end

declare extern type EnumPartType extends EnumItem with
end

declare extern type EnumPartType_INTERNAL extends Enum with
	Ball: EnumPartType
	Block: EnumPartType
	Cylinder: EnumPartType
	Wedge: EnumPartType
	CornerWedge: EnumPartType
	function GetEnumItems(self): { EnumPartType }
	function FromName(self, name: string): EnumPartType?
	function FromValue(self, value: number): EnumPartType?
end

declare extern type EnumParticleEmitterShape extends EnumItem with
end

declare extern type EnumParticleEmitterShape_INTERNAL extends Enum with
	Box: EnumParticleEmitterShape
	Sphere: EnumParticleEmitterShape
	Cylinder: EnumParticleEmitterShape
	Disc: EnumParticleEmitterShape
	function GetEnumItems(self): { EnumParticleEmitterShape }
	function FromName(self, name: string): EnumParticleEmitterShape?
	function FromValue(self, value: number): EnumParticleEmitterShape?
end

declare extern type EnumParticleEmitterShapeInOut extends EnumItem with
end

declare extern type EnumParticleEmitterShapeInOut_INTERNAL extends Enum with
	Outward: EnumParticleEmitterShapeInOut
	Inward: EnumParticleEmitterShapeInOut
	InAndOut: EnumParticleEmitterShapeInOut
	function GetEnumItems(self): { EnumParticleEmitterShapeInOut }
	function FromName(self, name: string): EnumParticleEmitterShapeInOut?
	function FromValue(self, value: number): EnumParticleEmitterShapeInOut?
end

declare extern type EnumParticleEmitterShapeStyle extends EnumItem with
end

declare extern type EnumParticleEmitterShapeStyle_INTERNAL extends Enum with
	Volume: EnumParticleEmitterShapeStyle
	Surface: EnumParticleEmitterShapeStyle
	function GetEnumItems(self): { EnumParticleEmitterShapeStyle }
	function FromName(self, name: string): EnumParticleEmitterShapeStyle?
	function FromValue(self, value: number): EnumParticleEmitterShapeStyle?
end

declare extern type EnumParticleFlipbookLayout extends EnumItem with
end

declare extern type EnumParticleFlipbookLayout_INTERNAL extends Enum with
	None: EnumParticleFlipbookLayout
	Grid2x2: EnumParticleFlipbookLayout
	Grid4x4: EnumParticleFlipbookLayout
	Grid8x8: EnumParticleFlipbookLayout
	Custom: EnumParticleFlipbookLayout
	function GetEnumItems(self): { EnumParticleFlipbookLayout }
	function FromName(self, name: string): EnumParticleFlipbookLayout?
	function FromValue(self, value: number): EnumParticleFlipbookLayout?
end

declare extern type EnumParticleFlipbookMode extends EnumItem with
end

declare extern type EnumParticleFlipbookMode_INTERNAL extends Enum with
	Loop: EnumParticleFlipbookMode
	OneShot: EnumParticleFlipbookMode
	PingPong: EnumParticleFlipbookMode
	Random: EnumParticleFlipbookMode
	function GetEnumItems(self): { EnumParticleFlipbookMode }
	function FromName(self, name: string): EnumParticleFlipbookMode?
	function FromValue(self, value: number): EnumParticleFlipbookMode?
end

declare extern type EnumParticleFlipbookTextureCompatible extends EnumItem with
end

declare extern type EnumParticleFlipbookTextureCompatible_INTERNAL extends Enum with
	NotCompatible: EnumParticleFlipbookTextureCompatible
	Compatible: EnumParticleFlipbookTextureCompatible
	Unknown: EnumParticleFlipbookTextureCompatible
	function GetEnumItems(self): { EnumParticleFlipbookTextureCompatible }
	function FromName(self, name: string): EnumParticleFlipbookTextureCompatible?
	function FromValue(self, value: number): EnumParticleFlipbookTextureCompatible?
end

declare extern type EnumParticleOrientation extends EnumItem with
end

declare extern type EnumParticleOrientation_INTERNAL extends Enum with
	FacingCamera: EnumParticleOrientation
	FacingCameraWorldUp: EnumParticleOrientation
	VelocityParallel: EnumParticleOrientation
	VelocityPerpendicular: EnumParticleOrientation
	function GetEnumItems(self): { EnumParticleOrientation }
	function FromName(self, name: string): EnumParticleOrientation?
	function FromValue(self, value: number): EnumParticleOrientation?
end

declare extern type EnumPathStatus extends EnumItem with
end

declare extern type EnumPathStatus_INTERNAL extends Enum with
	Success: EnumPathStatus
	ClosestNoPath: EnumPathStatus
	ClosestOutOfRange: EnumPathStatus
	FailStartNotEmpty: EnumPathStatus
	FailFinishNotEmpty: EnumPathStatus
	NoPath: EnumPathStatus
	function GetEnumItems(self): { EnumPathStatus }
	function FromName(self, name: string): EnumPathStatus?
	function FromValue(self, value: number): EnumPathStatus?
end

declare extern type EnumPathWaypointAction extends EnumItem with
end

declare extern type EnumPathWaypointAction_INTERNAL extends Enum with
	Walk: EnumPathWaypointAction
	Jump: EnumPathWaypointAction
	Custom: EnumPathWaypointAction
	function GetEnumItems(self): { EnumPathWaypointAction }
	function FromName(self, name: string): EnumPathWaypointAction?
	function FromValue(self, value: number): EnumPathWaypointAction?
end

declare extern type EnumPathfindingUseImprovedSearch extends EnumItem with
end

declare extern type EnumPathfindingUseImprovedSearch_INTERNAL extends Enum with
	Default: EnumPathfindingUseImprovedSearch
	Disabled: EnumPathfindingUseImprovedSearch
	Enabled: EnumPathfindingUseImprovedSearch
	function GetEnumItems(self): { EnumPathfindingUseImprovedSearch }
	function FromName(self, name: string): EnumPathfindingUseImprovedSearch?
	function FromValue(self, value: number): EnumPathfindingUseImprovedSearch?
end

declare extern type EnumPeoplePageLayout extends EnumItem with
end

declare extern type EnumPeoplePageLayout_INTERNAL extends Enum with
	Card: EnumPeoplePageLayout
	List: EnumPeoplePageLayout
	function GetEnumItems(self): { EnumPeoplePageLayout }
	function FromName(self, name: string): EnumPeoplePageLayout?
	function FromValue(self, value: number): EnumPeoplePageLayout?
end

declare extern type EnumPerformanceOverlayMode extends EnumItem with
end

declare extern type EnumPerformanceOverlayMode_INTERNAL extends Enum with
	Overdraw: EnumPerformanceOverlayMode
	Transparent: EnumPerformanceOverlayMode
	Decals: EnumPerformanceOverlayMode
	Lights: EnumPerformanceOverlayMode
	function GetEnumItems(self): { EnumPerformanceOverlayMode }
	function FromName(self, name: string): EnumPerformanceOverlayMode?
	function FromValue(self, value: number): EnumPerformanceOverlayMode?
end

declare extern type EnumPermissionLevelShown extends EnumItem with
end

declare extern type EnumPermissionLevelShown_INTERNAL extends Enum with
	Game: EnumPermissionLevelShown
	RobloxGame: EnumPermissionLevelShown
	RobloxScript: EnumPermissionLevelShown
	Studio: EnumPermissionLevelShown
	Roblox: EnumPermissionLevelShown
	function GetEnumItems(self): { EnumPermissionLevelShown }
	function FromName(self, name: string): EnumPermissionLevelShown?
	function FromValue(self, value: number): EnumPermissionLevelShown?
end

declare extern type EnumPhysicalConstraintType extends EnumItem with
end

declare extern type EnumPhysicalConstraintType_INTERNAL extends Enum with
	AnimationConstraint: EnumPhysicalConstraintType
	Motor6D: EnumPhysicalConstraintType
	function GetEnumItems(self): { EnumPhysicalConstraintType }
	function FromName(self, name: string): EnumPhysicalConstraintType?
	function FromValue(self, value: number): EnumPhysicalConstraintType?
end

declare extern type EnumPhysicsSimulationRate extends EnumItem with
end

declare extern type EnumPhysicsSimulationRate_INTERNAL extends Enum with
	Fixed240Hz: EnumPhysicsSimulationRate
	Fixed120Hz: EnumPhysicsSimulationRate
	Fixed60Hz: EnumPhysicsSimulationRate
	function GetEnumItems(self): { EnumPhysicsSimulationRate }
	function FromName(self, name: string): EnumPhysicsSimulationRate?
	function FromValue(self, value: number): EnumPhysicsSimulationRate?
end

declare extern type EnumPhysicsSteppingMethod extends EnumItem with
end

declare extern type EnumPhysicsSteppingMethod_INTERNAL extends Enum with
	Default: EnumPhysicsSteppingMethod
	Fixed: EnumPhysicsSteppingMethod
	Adaptive: EnumPhysicsSteppingMethod
	function GetEnumItems(self): { EnumPhysicsSteppingMethod }
	function FromName(self, name: string): EnumPhysicsSteppingMethod?
	function FromValue(self, value: number): EnumPhysicsSteppingMethod?
end

declare extern type EnumPlaceContentPreference extends EnumItem with
end

declare extern type EnumPlaceContentPreference_INTERNAL extends Enum with
	None: EnumPlaceContentPreference
	All: EnumPlaceContentPreference
	MentionsAndReplies: EnumPlaceContentPreference
	Unknown: EnumPlaceContentPreference
	function GetEnumItems(self): { EnumPlaceContentPreference }
	function FromName(self, name: string): EnumPlaceContentPreference?
	function FromValue(self, value: number): EnumPlaceContentPreference?
end

declare extern type EnumPlacePublishType extends EnumItem with
end

declare extern type EnumPlacePublishType_INTERNAL extends Enum with
	None: EnumPlacePublishType
	Publish: EnumPlacePublishType
	Save: EnumPlacePublishType
	function GetEnumItems(self): { EnumPlacePublishType }
	function FromName(self, name: string): EnumPlacePublishType?
	function FromValue(self, value: number): EnumPlacePublishType?
end

declare extern type EnumPlatform extends EnumItem with
end

declare extern type EnumPlatform_INTERNAL extends Enum with
	Windows: EnumPlatform
	OSX: EnumPlatform
	IOS: EnumPlatform
	Android: EnumPlatform
	XBoxOne: EnumPlatform
	PS4: EnumPlatform
	PS3: EnumPlatform
	XBox360: EnumPlatform
	WiiU: EnumPlatform
	NX: EnumPlatform
	Ouya: EnumPlatform
	AndroidTV: EnumPlatform
	Chromecast: EnumPlatform
	Linux: EnumPlatform
	SteamOS: EnumPlatform
	WebOS: EnumPlatform
	DOS: EnumPlatform
	BeOS: EnumPlatform
	UWP: EnumPlatform
	PS5: EnumPlatform
	MetaOS: EnumPlatform
	Web: EnumPlatform
	None: EnumPlatform
	function GetEnumItems(self): { EnumPlatform }
	function FromName(self, name: string): EnumPlatform?
	function FromValue(self, value: number): EnumPlatform?
end

declare extern type EnumPlaybackState extends EnumItem with
end

declare extern type EnumPlaybackState_INTERNAL extends Enum with
	Begin: EnumPlaybackState
	Delayed: EnumPlaybackState
	Playing: EnumPlaybackState
	Paused: EnumPlaybackState
	Completed: EnumPlaybackState
	Cancelled: EnumPlaybackState
	function GetEnumItems(self): { EnumPlaybackState }
	function FromName(self, name: string): EnumPlaybackState?
	function FromValue(self, value: number): EnumPlaybackState?
end

declare extern type EnumPlayerActions extends EnumItem with
end

declare extern type EnumPlayerActions_INTERNAL extends Enum with
	CharacterForward: EnumPlayerActions
	CharacterBackward: EnumPlayerActions
	CharacterLeft: EnumPlayerActions
	CharacterRight: EnumPlayerActions
	CharacterJump: EnumPlayerActions
	function GetEnumItems(self): { EnumPlayerActions }
	function FromName(self, name: string): EnumPlayerActions?
	function FromValue(self, value: number): EnumPlayerActions?
end

declare extern type EnumPlayerCharacterDestroyBehavior extends EnumItem with
end

declare extern type EnumPlayerCharacterDestroyBehavior_INTERNAL extends Enum with
	Default: EnumPlayerCharacterDestroyBehavior
	Disabled: EnumPlayerCharacterDestroyBehavior
	Enabled: EnumPlayerCharacterDestroyBehavior
	function GetEnumItems(self): { EnumPlayerCharacterDestroyBehavior }
	function FromName(self, name: string): EnumPlayerCharacterDestroyBehavior?
	function FromValue(self, value: number): EnumPlayerCharacterDestroyBehavior?
end

declare extern type EnumPlayerChatType extends EnumItem with
end

declare extern type EnumPlayerChatType_INTERNAL extends Enum with
	All: EnumPlayerChatType
	Team: EnumPlayerChatType
	Whisper: EnumPlayerChatType
	function GetEnumItems(self): { EnumPlayerChatType }
	function FromName(self, name: string): EnumPlayerChatType?
	function FromValue(self, value: number): EnumPlayerChatType?
end

declare extern type EnumPlayerDataErrorState extends EnumItem with
end

declare extern type EnumPlayerDataErrorState_INTERNAL extends Enum with
	LoadFailed: EnumPlayerDataErrorState
	FlushFailed: EnumPlayerDataErrorState
	ReleaseFailed: EnumPlayerDataErrorState
	None: EnumPlayerDataErrorState
	function GetEnumItems(self): { EnumPlayerDataErrorState }
	function FromName(self, name: string): EnumPlayerDataErrorState?
	function FromValue(self, value: number): EnumPlayerDataErrorState?
end

declare extern type EnumPlayerDataLoadFailureBehavior extends EnumItem with
end

declare extern type EnumPlayerDataLoadFailureBehavior_INTERNAL extends Enum with
	Failure: EnumPlayerDataLoadFailureBehavior
	FallbackToDefault: EnumPlayerDataLoadFailureBehavior
	Kick: EnumPlayerDataLoadFailureBehavior
	function GetEnumItems(self): { EnumPlayerDataLoadFailureBehavior }
	function FromName(self, name: string): EnumPlayerDataLoadFailureBehavior?
	function FromValue(self, value: number): EnumPlayerDataLoadFailureBehavior?
end

declare extern type EnumPlayerExitReason extends EnumItem with
end

declare extern type EnumPlayerExitReason_INTERNAL extends Enum with
	Unknown: EnumPlayerExitReason
	PlatformKick: EnumPlayerExitReason
	CreatorKick: EnumPlayerExitReason
	function GetEnumItems(self): { EnumPlayerExitReason }
	function FromName(self, name: string): EnumPlayerExitReason?
	function FromValue(self, value: number): EnumPlayerExitReason?
end

declare extern type EnumPlayerPlatformActivationStatus extends EnumItem with
end

declare extern type EnumPlayerPlatformActivationStatus_INTERNAL extends Enum with
	Unknown: EnumPlayerPlatformActivationStatus
	New: EnumPlayerPlatformActivationStatus
	Active: EnumPlayerPlatformActivationStatus
	Lapsed: EnumPlayerPlatformActivationStatus
	Reactivated: EnumPlayerPlatformActivationStatus
	function GetEnumItems(self): { EnumPlayerPlatformActivationStatus }
	function FromName(self, name: string): EnumPlayerPlatformActivationStatus?
	function FromValue(self, value: number): EnumPlayerPlatformActivationStatus?
end

declare extern type EnumPlayerPlatformSpenderStatus extends EnumItem with
end

declare extern type EnumPlayerPlatformSpenderStatus_INTERNAL extends Enum with
	Unknown: EnumPlayerPlatformSpenderStatus
	Active: EnumPlayerPlatformSpenderStatus
	OtherPayer: EnumPlayerPlatformSpenderStatus
	function GetEnumItems(self): { EnumPlayerPlatformSpenderStatus }
	function FromName(self, name: string): EnumPlayerPlatformSpenderStatus?
	function FromValue(self, value: number): EnumPlayerPlatformSpenderStatus?
end

declare extern type EnumPluginConnectionTargetType extends EnumItem with
end

declare extern type EnumPluginConnectionTargetType_INTERNAL extends Enum with
	Edit: EnumPluginConnectionTargetType
	Test: EnumPluginConnectionTargetType
	function GetEnumItems(self): { EnumPluginConnectionTargetType }
	function FromName(self, name: string): EnumPluginConnectionTargetType?
	function FromValue(self, value: number): EnumPluginConnectionTargetType?
end

declare extern type EnumPoseEasingDirection extends EnumItem with
end

declare extern type EnumPoseEasingDirection_INTERNAL extends Enum with
	In: EnumPoseEasingDirection
	Out: EnumPoseEasingDirection
	InOut: EnumPoseEasingDirection
	function GetEnumItems(self): { EnumPoseEasingDirection }
	function FromName(self, name: string): EnumPoseEasingDirection?
	function FromValue(self, value: number): EnumPoseEasingDirection?
end

declare extern type EnumPoseEasingStyle extends EnumItem with
end

declare extern type EnumPoseEasingStyle_INTERNAL extends Enum with
	Linear: EnumPoseEasingStyle
	Constant: EnumPoseEasingStyle
	Elastic: EnumPoseEasingStyle
	Cubic: EnumPoseEasingStyle
	Bounce: EnumPoseEasingStyle
	CubicV2: EnumPoseEasingStyle
	function GetEnumItems(self): { EnumPoseEasingStyle }
	function FromName(self, name: string): EnumPoseEasingStyle?
	function FromValue(self, value: number): EnumPoseEasingStyle?
end

declare extern type EnumPositionAlignmentMode extends EnumItem with
end

declare extern type EnumPositionAlignmentMode_INTERNAL extends Enum with
	OneAttachment: EnumPositionAlignmentMode
	TwoAttachment: EnumPositionAlignmentMode
	function GetEnumItems(self): { EnumPositionAlignmentMode }
	function FromName(self, name: string): EnumPositionAlignmentMode?
	function FromValue(self, value: number): EnumPositionAlignmentMode?
end

declare extern type EnumPredictionMode extends EnumItem with
end

declare extern type EnumPredictionMode_INTERNAL extends Enum with
	Automatic: EnumPredictionMode
	On: EnumPredictionMode
	Off: EnumPredictionMode
	function GetEnumItems(self): { EnumPredictionMode }
	function FromName(self, name: string): EnumPredictionMode?
	function FromValue(self, value: number): EnumPredictionMode?
end

declare extern type EnumPredictionStatus extends EnumItem with
end

declare extern type EnumPredictionStatus_INTERNAL extends Enum with
	Authoritative: EnumPredictionStatus
	Predicted: EnumPredictionStatus
	None: EnumPredictionStatus
	function GetEnumItems(self): { EnumPredictionStatus }
	function FromName(self, name: string): EnumPredictionStatus?
	function FromValue(self, value: number): EnumPredictionStatus?
end

declare extern type EnumPreferredInput extends EnumItem with
end

declare extern type EnumPreferredInput_INTERNAL extends Enum with
	KeyboardAndMouse: EnumPreferredInput
	Gamepad: EnumPreferredInput
	Touch: EnumPreferredInput
	function GetEnumItems(self): { EnumPreferredInput }
	function FromName(self, name: string): EnumPreferredInput?
	function FromValue(self, value: number): EnumPreferredInput?
end

declare extern type EnumPreferredTextSize extends EnumItem with
end

declare extern type EnumPreferredTextSize_INTERNAL extends Enum with
	Medium: EnumPreferredTextSize
	Large: EnumPreferredTextSize
	Larger: EnumPreferredTextSize
	Largest: EnumPreferredTextSize
	function GetEnumItems(self): { EnumPreferredTextSize }
	function FromName(self, name: string): EnumPreferredTextSize?
	function FromValue(self, value: number): EnumPreferredTextSize?
end

declare extern type EnumPrefetchDownloadStatus extends EnumItem with
end

declare extern type EnumPrefetchDownloadStatus_INTERNAL extends Enum with
	NotStarted: EnumPrefetchDownloadStatus
	InProgress: EnumPrefetchDownloadStatus
	Completed: EnumPrefetchDownloadStatus
	Failed: EnumPrefetchDownloadStatus
	function GetEnumItems(self): { EnumPrefetchDownloadStatus }
	function FromName(self, name: string): EnumPrefetchDownloadStatus?
	function FromValue(self, value: number): EnumPrefetchDownloadStatus?
end

declare extern type EnumPrimalPhysicsSolver extends EnumItem with
end

declare extern type EnumPrimalPhysicsSolver_INTERNAL extends Enum with
	Default: EnumPrimalPhysicsSolver
	Experimental: EnumPrimalPhysicsSolver
	Disabled: EnumPrimalPhysicsSolver
	function GetEnumItems(self): { EnumPrimalPhysicsSolver }
	function FromName(self, name: string): EnumPrimalPhysicsSolver?
	function FromValue(self, value: number): EnumPrimalPhysicsSolver?
end

declare extern type EnumPrimitiveType extends EnumItem with
end

declare extern type EnumPrimitiveType_INTERNAL extends Enum with
	Null: EnumPrimitiveType
	Ball: EnumPrimitiveType
	Cylinder: EnumPrimitiveType
	Block: EnumPrimitiveType
	Wedge: EnumPrimitiveType
	CornerWedge: EnumPrimitiveType
	function GetEnumItems(self): { EnumPrimitiveType }
	function FromName(self, name: string): EnumPrimitiveType?
	function FromValue(self, value: number): EnumPrimitiveType?
end

declare extern type EnumPrivilegeType extends EnumItem with
end

declare extern type EnumPrivilegeType_INTERNAL extends Enum with
	Banned: EnumPrivilegeType
	Visitor: EnumPrivilegeType
	Member: EnumPrivilegeType
	Admin: EnumPrivilegeType
	Owner: EnumPrivilegeType
	function GetEnumItems(self): { EnumPrivilegeType }
	function FromName(self, name: string): EnumPrivilegeType?
	function FromValue(self, value: number): EnumPrivilegeType?
end

declare extern type EnumProductLocationRestriction extends EnumItem with
end

declare extern type EnumProductLocationRestriction_INTERNAL extends Enum with
	AvatarShop: EnumProductLocationRestriction
	AllowedGames: EnumProductLocationRestriction
	AllGames: EnumProductLocationRestriction
	function GetEnumItems(self): { EnumProductLocationRestriction }
	function FromName(self, name: string): EnumProductLocationRestriction?
	function FromValue(self, value: number): EnumProductLocationRestriction?
end

declare extern type EnumProductPurchaseChannel extends EnumItem with
end

declare extern type EnumProductPurchaseChannel_INTERNAL extends Enum with
	InExperience: EnumProductPurchaseChannel
	ExperienceDetailsPage: EnumProductPurchaseChannel
	AdReward: EnumProductPurchaseChannel
	CommerceProduct: EnumProductPurchaseChannel
	function GetEnumItems(self): { EnumProductPurchaseChannel }
	function FromName(self, name: string): EnumProductPurchaseChannel?
	function FromValue(self, value: number): EnumProductPurchaseChannel?
end

declare extern type EnumProductPurchaseDecision extends EnumItem with
end

declare extern type EnumProductPurchaseDecision_INTERNAL extends Enum with
	NotProcessedYet: EnumProductPurchaseDecision
	PurchaseGranted: EnumProductPurchaseDecision
	function GetEnumItems(self): { EnumProductPurchaseDecision }
	function FromName(self, name: string): EnumProductPurchaseDecision?
	function FromValue(self, value: number): EnumProductPurchaseDecision?
end

declare extern type EnumPromptCreateAssetResult extends EnumItem with
end

declare extern type EnumPromptCreateAssetResult_INTERNAL extends Enum with
	Success: EnumPromptCreateAssetResult
	PermissionDenied: EnumPromptCreateAssetResult
	Timeout: EnumPromptCreateAssetResult
	UploadFailed: EnumPromptCreateAssetResult
	NoUserInput: EnumPromptCreateAssetResult
	UnknownFailure: EnumPromptCreateAssetResult
	UGCValidationFailed: EnumPromptCreateAssetResult
	ModeratedName: EnumPromptCreateAssetResult
	PurchaseFailure: EnumPromptCreateAssetResult
	TokenInvalid: EnumPromptCreateAssetResult
	function GetEnumItems(self): { EnumPromptCreateAssetResult }
	function FromName(self, name: string): EnumPromptCreateAssetResult?
	function FromValue(self, value: number): EnumPromptCreateAssetResult?
end

declare extern type EnumPromptCreateAvatarResult extends EnumItem with
end

declare extern type EnumPromptCreateAvatarResult_INTERNAL extends Enum with
	Success: EnumPromptCreateAvatarResult
	PermissionDenied: EnumPromptCreateAvatarResult
	Timeout: EnumPromptCreateAvatarResult
	UploadFailed: EnumPromptCreateAvatarResult
	NoUserInput: EnumPromptCreateAvatarResult
	InvalidHumanoidDescription: EnumPromptCreateAvatarResult
	UGCValidationFailed: EnumPromptCreateAvatarResult
	ModeratedName: EnumPromptCreateAvatarResult
	MaxOutfits: EnumPromptCreateAvatarResult
	PurchaseFailure: EnumPromptCreateAvatarResult
	UnknownFailure: EnumPromptCreateAvatarResult
	TokenInvalid: EnumPromptCreateAvatarResult
	function GetEnumItems(self): { EnumPromptCreateAvatarResult }
	function FromName(self, name: string): EnumPromptCreateAvatarResult?
	function FromValue(self, value: number): EnumPromptCreateAvatarResult?
end

declare extern type EnumPromptExperienceDetailsResult extends EnumItem with
end

declare extern type EnumPromptExperienceDetailsResult_INTERNAL extends Enum with
	PromptClosed: EnumPromptExperienceDetailsResult
	TeleportAttempted: EnumPromptExperienceDetailsResult
	function GetEnumItems(self): { EnumPromptExperienceDetailsResult }
	function FromName(self, name: string): EnumPromptExperienceDetailsResult?
	function FromValue(self, value: number): EnumPromptExperienceDetailsResult?
end

declare extern type EnumPromptLinkSharingResult extends EnumItem with
end

declare extern type EnumPromptLinkSharingResult_INTERNAL extends Enum with
	Success: EnumPromptLinkSharingResult
	PlayerLeft: EnumPromptLinkSharingResult
	InvalidLaunchData: EnumPromptLinkSharingResult
	function GetEnumItems(self): { EnumPromptLinkSharingResult }
	function FromName(self, name: string): EnumPromptLinkSharingResult?
	function FromValue(self, value: number): EnumPromptLinkSharingResult?
end

declare extern type EnumPromptPublishAssetResult extends EnumItem with
end

declare extern type EnumPromptPublishAssetResult_INTERNAL extends Enum with
	Success: EnumPromptPublishAssetResult
	PermissionDenied: EnumPromptPublishAssetResult
	Timeout: EnumPromptPublishAssetResult
	UploadFailed: EnumPromptPublishAssetResult
	NoUserInput: EnumPromptPublishAssetResult
	UnknownFailure: EnumPromptPublishAssetResult
	function GetEnumItems(self): { EnumPromptPublishAssetResult }
	function FromName(self, name: string): EnumPromptPublishAssetResult?
	function FromValue(self, value: number): EnumPromptPublishAssetResult?
end

declare extern type EnumPropertyStatus extends EnumItem with
end

declare extern type EnumPropertyStatus_INTERNAL extends Enum with
	Ok: EnumPropertyStatus
	Warning: EnumPropertyStatus
	Error: EnumPropertyStatus
	function GetEnumItems(self): { EnumPropertyStatus }
	function FromName(self, name: string): EnumPropertyStatus?
	function FromValue(self, value: number): EnumPropertyStatus?
end

declare extern type EnumProximityPromptExclusivity extends EnumItem with
end

declare extern type EnumProximityPromptExclusivity_INTERNAL extends Enum with
	OnePerButton: EnumProximityPromptExclusivity
	OneGlobally: EnumProximityPromptExclusivity
	AlwaysShow: EnumProximityPromptExclusivity
	function GetEnumItems(self): { EnumProximityPromptExclusivity }
	function FromName(self, name: string): EnumProximityPromptExclusivity?
	function FromValue(self, value: number): EnumProximityPromptExclusivity?
end

declare extern type EnumProximityPromptInputType extends EnumItem with
end

declare extern type EnumProximityPromptInputType_INTERNAL extends Enum with
	Keyboard: EnumProximityPromptInputType
	Gamepad: EnumProximityPromptInputType
	Touch: EnumProximityPromptInputType
	function GetEnumItems(self): { EnumProximityPromptInputType }
	function FromName(self, name: string): EnumProximityPromptInputType?
	function FromValue(self, value: number): EnumProximityPromptInputType?
end

declare extern type EnumProximityPromptStyle extends EnumItem with
end

declare extern type EnumProximityPromptStyle_INTERNAL extends Enum with
	Default: EnumProximityPromptStyle
	Custom: EnumProximityPromptStyle
	function GetEnumItems(self): { EnumProximityPromptStyle }
	function FromName(self, name: string): EnumProximityPromptStyle?
	function FromValue(self, value: number): EnumProximityPromptStyle?
end

declare extern type EnumPurchaseOption extends EnumItem with
end

declare extern type EnumPurchaseOption_INTERNAL extends Enum with
	TimedOption: EnumPurchaseOption
	Permanent: EnumPurchaseOption
	function GetEnumItems(self): { EnumPurchaseOption }
	function FromName(self, name: string): EnumPurchaseOption?
	function FromValue(self, value: number): EnumPurchaseOption?
end

declare extern type EnumQualityLevel extends EnumItem with
end

declare extern type EnumQualityLevel_INTERNAL extends Enum with
	Automatic: EnumQualityLevel
	Level01: EnumQualityLevel
	Level02: EnumQualityLevel
	Level03: EnumQualityLevel
	Level04: EnumQualityLevel
	Level05: EnumQualityLevel
	Level06: EnumQualityLevel
	Level07: EnumQualityLevel
	Level08: EnumQualityLevel
	Level09: EnumQualityLevel
	Level10: EnumQualityLevel
	Level11: EnumQualityLevel
	Level12: EnumQualityLevel
	Level13: EnumQualityLevel
	Level14: EnumQualityLevel
	Level15: EnumQualityLevel
	Level16: EnumQualityLevel
	Level17: EnumQualityLevel
	Level18: EnumQualityLevel
	Level19: EnumQualityLevel
	Level20: EnumQualityLevel
	Level21: EnumQualityLevel
	function GetEnumItems(self): { EnumQualityLevel }
	function FromName(self, name: string): EnumQualityLevel?
	function FromValue(self, value: number): EnumQualityLevel?
end

declare extern type EnumR15CollisionType extends EnumItem with
end

declare extern type EnumR15CollisionType_INTERNAL extends Enum with
	OuterBox: EnumR15CollisionType
	InnerBox: EnumR15CollisionType
	function GetEnumItems(self): { EnumR15CollisionType }
	function FromName(self, name: string): EnumR15CollisionType?
	function FromValue(self, value: number): EnumR15CollisionType?
end

declare extern type EnumRaycastFilterType extends EnumItem with
end

declare extern type EnumRaycastFilterType_INTERNAL extends Enum with
	Exclude: EnumRaycastFilterType
	Include: EnumRaycastFilterType
	function GetEnumItems(self): { EnumRaycastFilterType }
	function FromName(self, name: string): EnumRaycastFilterType?
	function FromValue(self, value: number): EnumRaycastFilterType?
end

declare extern type EnumReadCapturesFromGalleryResult extends EnumItem with
end

declare extern type EnumReadCapturesFromGalleryResult_INTERNAL extends Enum with
	Success: EnumReadCapturesFromGalleryResult
	NeedPermission: EnumReadCapturesFromGalleryResult
	function GetEnumItems(self): { EnumReadCapturesFromGalleryResult }
	function FromName(self, name: string): EnumReadCapturesFromGalleryResult?
	function FromValue(self, value: number): EnumReadCapturesFromGalleryResult?
end

declare extern type EnumReceiptDecision extends EnumItem with
end

declare extern type EnumReceiptDecision_INTERNAL extends Enum with
	NotProcessedYet: EnumReceiptDecision
	Processed: EnumReceiptDecision
	function GetEnumItems(self): { EnumReceiptDecision }
	function FromName(self, name: string): EnumReceiptDecision?
	function FromValue(self, value: number): EnumReceiptDecision?
end

declare extern type EnumReceiptType extends EnumItem with
end

declare extern type EnumReceiptType_INTERNAL extends Enum with
	DeveloperProduct: EnumReceiptType
	RobuxTransferSender: EnumReceiptType
	RobuxTransferReceiver: EnumReceiptType
	function GetEnumItems(self): { EnumReceiptType }
	function FromName(self, name: string): EnumReceiptType?
	function FromValue(self, value: number): EnumReceiptType?
end

declare extern type EnumRecommendationActionType extends EnumItem with
end

declare extern type EnumRecommendationActionType_INTERNAL extends Enum with
	AddReaction: EnumRecommendationActionType
	RemoveReaction: EnumRecommendationActionType
	Share: EnumRecommendationActionType
	Report: EnumRecommendationActionType
	Comment: EnumRecommendationActionType
	Play: EnumRecommendationActionType
	Purchase: EnumRecommendationActionType
	function GetEnumItems(self): { EnumRecommendationActionType }
	function FromName(self, name: string): EnumRecommendationActionType?
	function FromValue(self, value: number): EnumRecommendationActionType?
end

declare extern type EnumRecommendationDepartureIntent extends EnumItem with
end

declare extern type EnumRecommendationDepartureIntent_INTERNAL extends Enum with
	Neutral: EnumRecommendationDepartureIntent
	Positive: EnumRecommendationDepartureIntent
	Negative: EnumRecommendationDepartureIntent
	function GetEnumItems(self): { EnumRecommendationDepartureIntent }
	function FromName(self, name: string): EnumRecommendationDepartureIntent?
	function FromValue(self, value: number): EnumRecommendationDepartureIntent?
end

declare extern type EnumRecommendationImpressionType extends EnumItem with
end

declare extern type EnumRecommendationImpressionType_INTERNAL extends Enum with
	View: EnumRecommendationImpressionType
	NotViewable: EnumRecommendationImpressionType
	function GetEnumItems(self): { EnumRecommendationImpressionType }
	function FromName(self, name: string): EnumRecommendationImpressionType?
	function FromValue(self, value: number): EnumRecommendationImpressionType?
end

declare extern type EnumRecommendationItemContentType extends EnumItem with
end

declare extern type EnumRecommendationItemContentType_INTERNAL extends Enum with
	Static: EnumRecommendationItemContentType
	Dynamic: EnumRecommendationItemContentType
	Interactive: EnumRecommendationItemContentType
	function GetEnumItems(self): { EnumRecommendationItemContentType }
	function FromName(self, name: string): EnumRecommendationItemContentType?
	function FromValue(self, value: number): EnumRecommendationItemContentType?
end

declare extern type EnumRecommendationItemVisibility extends EnumItem with
end

declare extern type EnumRecommendationItemVisibility_INTERNAL extends Enum with
	Private: EnumRecommendationItemVisibility
	Public: EnumRecommendationItemVisibility
	function GetEnumItems(self): { EnumRecommendationItemVisibility }
	function FromName(self, name: string): EnumRecommendationItemVisibility?
	function FromValue(self, value: number): EnumRecommendationItemVisibility?
end

declare extern type EnumRecommendationPreferenceTargetType extends EnumItem with
end

declare extern type EnumRecommendationPreferenceTargetType_INTERNAL extends Enum with
	User: EnumRecommendationPreferenceTargetType
	Universe: EnumRecommendationPreferenceTargetType
	CustomTag: EnumRecommendationPreferenceTargetType
	function GetEnumItems(self): { EnumRecommendationPreferenceTargetType }
	function FromName(self, name: string): EnumRecommendationPreferenceTargetType?
	function FromValue(self, value: number): EnumRecommendationPreferenceTargetType?
end

declare extern type EnumRecommendationPreferenceType extends EnumItem with
end

declare extern type EnumRecommendationPreferenceType_INTERNAL extends Enum with
	AddFollow: EnumRecommendationPreferenceType
	RemoveFollow: EnumRecommendationPreferenceType
	AddMute: EnumRecommendationPreferenceType
	RemoveMute: EnumRecommendationPreferenceType
	function GetEnumItems(self): { EnumRecommendationPreferenceType }
	function FromName(self, name: string): EnumRecommendationPreferenceType?
	function FromValue(self, value: number): EnumRecommendationPreferenceType?
end

declare extern type EnumRejectCharacterDeletions extends EnumItem with
end

declare extern type EnumRejectCharacterDeletions_INTERNAL extends Enum with
	Default: EnumRejectCharacterDeletions
	Disabled: EnumRejectCharacterDeletions
	Enabled: EnumRejectCharacterDeletions
	function GetEnumItems(self): { EnumRejectCharacterDeletions }
	function FromName(self, name: string): EnumRejectCharacterDeletions?
	function FromValue(self, value: number): EnumRejectCharacterDeletions?
end

declare extern type EnumRenderFidelity extends EnumItem with
end

declare extern type EnumRenderFidelity_INTERNAL extends Enum with
	Automatic: EnumRenderFidelity
	Precise: EnumRenderFidelity
	Performance: EnumRenderFidelity
	function GetEnumItems(self): { EnumRenderFidelity }
	function FromName(self, name: string): EnumRenderFidelity?
	function FromValue(self, value: number): EnumRenderFidelity?
end

declare extern type EnumRenderPriority extends EnumItem with
end

declare extern type EnumRenderPriority_INTERNAL extends Enum with
	First: EnumRenderPriority
	Input: EnumRenderPriority
	Camera: EnumRenderPriority
	Character: EnumRenderPriority
	Last: EnumRenderPriority
	function GetEnumItems(self): { EnumRenderPriority }
	function FromName(self, name: string): EnumRenderPriority?
	function FromValue(self, value: number): EnumRenderPriority?
end

declare extern type EnumRenderingCacheOptimizationMode extends EnumItem with
end

declare extern type EnumRenderingCacheOptimizationMode_INTERNAL extends Enum with
	Default: EnumRenderingCacheOptimizationMode
	Disabled: EnumRenderingCacheOptimizationMode
	Enabled: EnumRenderingCacheOptimizationMode
	function GetEnumItems(self): { EnumRenderingCacheOptimizationMode }
	function FromName(self, name: string): EnumRenderingCacheOptimizationMode?
	function FromValue(self, value: number): EnumRenderingCacheOptimizationMode?
end

declare extern type EnumRenderingTestComparisonMethod extends EnumItem with
end

declare extern type EnumRenderingTestComparisonMethod_INTERNAL extends Enum with
	psnr: EnumRenderingTestComparisonMethod
	diff: EnumRenderingTestComparisonMethod
	function GetEnumItems(self): { EnumRenderingTestComparisonMethod }
	function FromName(self, name: string): EnumRenderingTestComparisonMethod?
	function FromValue(self, value: number): EnumRenderingTestComparisonMethod?
end

declare extern type EnumReplicateInstanceDestroySetting extends EnumItem with
end

declare extern type EnumReplicateInstanceDestroySetting_INTERNAL extends Enum with
	Default: EnumReplicateInstanceDestroySetting
	Disabled: EnumReplicateInstanceDestroySetting
	Enabled: EnumReplicateInstanceDestroySetting
	function GetEnumItems(self): { EnumReplicateInstanceDestroySetting }
	function FromName(self, name: string): EnumReplicateInstanceDestroySetting?
	function FromValue(self, value: number): EnumReplicateInstanceDestroySetting?
end

declare extern type EnumResamplerMode extends EnumItem with
end

declare extern type EnumResamplerMode_INTERNAL extends Enum with
	Default: EnumResamplerMode
	Pixelated: EnumResamplerMode
	function GetEnumItems(self): { EnumResamplerMode }
	function FromName(self, name: string): EnumResamplerMode?
	function FromValue(self, value: number): EnumResamplerMode?
end

declare extern type EnumReservedHighlightId extends EnumItem with
end

declare extern type EnumReservedHighlightId_INTERNAL extends Enum with
	Standard: EnumReservedHighlightId
	Active: EnumReservedHighlightId
	Hover: EnumReservedHighlightId
	Selection: EnumReservedHighlightId
	NegatedPart: EnumReservedHighlightId
	function GetEnumItems(self): { EnumReservedHighlightId }
	function FromName(self, name: string): EnumReservedHighlightId?
	function FromValue(self, value: number): EnumReservedHighlightId?
end

declare extern type EnumRestPose extends EnumItem with
end

declare extern type EnumRestPose_INTERNAL extends Enum with
	Default: EnumRestPose
	RotationsReset: EnumRestPose
	Custom: EnumRestPose
	function GetEnumItems(self): { EnumRestPose }
	function FromName(self, name: string): EnumRestPose?
	function FromValue(self, value: number): EnumRestPose?
end

declare extern type EnumRestPoseModel extends EnumItem with
end

declare extern type EnumRestPoseModel_INTERNAL extends Enum with
	FromRigInACE: EnumRestPoseModel
	FromRigInFile: EnumRestPoseModel
	FromRigInFileZeroedRotations: EnumRestPoseModel
	FromCustomClip: EnumRestPoseModel
	function GetEnumItems(self): { EnumRestPoseModel }
	function FromName(self, name: string): EnumRestPoseModel?
	function FromValue(self, value: number): EnumRestPoseModel?
end

declare extern type EnumReturnKeyType extends EnumItem with
end

declare extern type EnumReturnKeyType_INTERNAL extends Enum with
	Default: EnumReturnKeyType
	Done: EnumReturnKeyType
	Go: EnumReturnKeyType
	Next: EnumReturnKeyType
	Search: EnumReturnKeyType
	Send: EnumReturnKeyType
	function GetEnumItems(self): { EnumReturnKeyType }
	function FromName(self, name: string): EnumReturnKeyType?
	function FromValue(self, value: number): EnumReturnKeyType?
end

declare extern type EnumReverbType extends EnumItem with
end

declare extern type EnumReverbType_INTERNAL extends Enum with
	NoReverb: EnumReverbType
	GenericReverb: EnumReverbType
	PaddedCell: EnumReverbType
	Room: EnumReverbType
	Bathroom: EnumReverbType
	LivingRoom: EnumReverbType
	StoneRoom: EnumReverbType
	Auditorium: EnumReverbType
	ConcertHall: EnumReverbType
	Cave: EnumReverbType
	Arena: EnumReverbType
	Hangar: EnumReverbType
	CarpettedHallway: EnumReverbType
	Hallway: EnumReverbType
	StoneCorridor: EnumReverbType
	Alley: EnumReverbType
	Forest: EnumReverbType
	City: EnumReverbType
	Mountains: EnumReverbType
	Quarry: EnumReverbType
	Plain: EnumReverbType
	ParkingLot: EnumReverbType
	SewerPipe: EnumReverbType
	UnderWater: EnumReverbType
	function GetEnumItems(self): { EnumReverbType }
	function FromName(self, name: string): EnumReverbType?
	function FromValue(self, value: number): EnumReverbType?
end

declare extern type EnumReviewableContentState extends EnumItem with
end

declare extern type EnumReviewableContentState_INTERNAL extends Enum with
	Pending: EnumReviewableContentState
	Completed: EnumReviewableContentState
	Failed: EnumReviewableContentState
	function GetEnumItems(self): { EnumReviewableContentState }
	function FromName(self, name: string): EnumReviewableContentState?
	function FromValue(self, value: number): EnumReviewableContentState?
end

declare extern type EnumRibbonTool extends EnumItem with
end

declare extern type EnumRibbonTool_INTERNAL extends Enum with
	Select: EnumRibbonTool
	Scale: EnumRibbonTool
	Rotate: EnumRibbonTool
	Move: EnumRibbonTool
	Transform: EnumRibbonTool
	ColorPicker: EnumRibbonTool
	MaterialPicker: EnumRibbonTool
	Group: EnumRibbonTool
	Ungroup: EnumRibbonTool
	None: EnumRibbonTool
	PivotEditor: EnumRibbonTool
	function GetEnumItems(self): { EnumRibbonTool }
	function FromName(self, name: string): EnumRibbonTool?
	function FromValue(self, value: number): EnumRibbonTool?
end

declare extern type EnumRigLabel extends EnumItem with
end

declare extern type EnumRigLabel_INTERNAL extends Enum with
	Invalid: EnumRigLabel
	Root: EnumRigLabel
	LeftHip: EnumRigLabel
	LeftKnee: EnumRigLabel
	LeftAnkle: EnumRigLabel
	RightHip: EnumRigLabel
	RightKnee: EnumRigLabel
	RightAnkle: EnumRigLabel
	Waist: EnumRigLabel
	LeftShoulder: EnumRigLabel
	LeftElbow: EnumRigLabel
	LeftWrist: EnumRigLabel
	RightShoulder: EnumRigLabel
	RightElbow: EnumRigLabel
	RightWrist: EnumRigLabel
	Neck: EnumRigLabel
	Spine: EnumRigLabel
	Chest: EnumRigLabel
	HeadBase: EnumRigLabel
	LeftClavicle: EnumRigLabel
	RightClavicle: EnumRigLabel
	LeftToeBase: EnumRigLabel
	RightToeBase: EnumRigLabel
	Thumb1: EnumRigLabel
	Thumb2: EnumRigLabel
	Thumb3: EnumRigLabel
	Index1: EnumRigLabel
	Index2: EnumRigLabel
	Index3: EnumRigLabel
	Middle1: EnumRigLabel
	Middle2: EnumRigLabel
	Middle3: EnumRigLabel
	Ring1: EnumRigLabel
	Ring2: EnumRigLabel
	Ring3: EnumRigLabel
	Pinky1: EnumRigLabel
	Pinky2: EnumRigLabel
	Pinky3: EnumRigLabel
	function GetEnumItems(self): { EnumRigLabel }
	function FromName(self, name: string): EnumRigLabel?
	function FromValue(self, value: number): EnumRigLabel?
end

declare extern type EnumRigScale extends EnumItem with
end

declare extern type EnumRigScale_INTERNAL extends Enum with
	Default: EnumRigScale
	Rthro: EnumRigScale
	RthroNarrow: EnumRigScale
	function GetEnumItems(self): { EnumRigScale }
	function FromName(self, name: string): EnumRigScale?
	function FromValue(self, value: number): EnumRigScale?
end

declare extern type EnumRigType extends EnumItem with
end

declare extern type EnumRigType_INTERNAL extends Enum with
	R15: EnumRigType
	CustomHumanoid: EnumRigType
	Custom: EnumRigType
	None: EnumRigType
	function GetEnumItems(self): { EnumRigType }
	function FromName(self, name: string): EnumRigType?
	function FromValue(self, value: number): EnumRigType?
end

declare extern type EnumRollOffMode extends EnumItem with
end

declare extern type EnumRollOffMode_INTERNAL extends Enum with
	Inverse: EnumRollOffMode
	Linear: EnumRollOffMode
	LinearSquare: EnumRollOffMode
	InverseTapered: EnumRollOffMode
	function GetEnumItems(self): { EnumRollOffMode }
	function FromName(self, name: string): EnumRollOffMode?
	function FromValue(self, value: number): EnumRollOffMode?
end

declare extern type EnumRolloutState extends EnumItem with
end

declare extern type EnumRolloutState_INTERNAL extends Enum with
	Default: EnumRolloutState
	Disabled: EnumRolloutState
	Enabled: EnumRolloutState
	function GetEnumItems(self): { EnumRolloutState }
	function FromName(self, name: string): EnumRolloutState?
	function FromValue(self, value: number): EnumRolloutState?
end

declare extern type EnumRotationOrder extends EnumItem with
end

declare extern type EnumRotationOrder_INTERNAL extends Enum with
	XYZ: EnumRotationOrder
	XZY: EnumRotationOrder
	YZX: EnumRotationOrder
	YXZ: EnumRotationOrder
	ZXY: EnumRotationOrder
	ZYX: EnumRotationOrder
	function GetEnumItems(self): { EnumRotationOrder }
	function FromName(self, name: string): EnumRotationOrder?
	function FromValue(self, value: number): EnumRotationOrder?
end

declare extern type EnumRotationType extends EnumItem with
end

declare extern type EnumRotationType_INTERNAL extends Enum with
	MovementRelative: EnumRotationType
	CameraRelative: EnumRotationType
	function GetEnumItems(self): { EnumRotationType }
	function FromName(self, name: string): EnumRotationType?
	function FromValue(self, value: number): EnumRotationType?
end

declare extern type EnumRsvpStatus extends EnumItem with
end

declare extern type EnumRsvpStatus_INTERNAL extends Enum with
	None: EnumRsvpStatus
	Going: EnumRsvpStatus
	NotGoing: EnumRsvpStatus
	function GetEnumItems(self): { EnumRsvpStatus }
	function FromName(self, name: string): EnumRsvpStatus?
	function FromValue(self, value: number): EnumRsvpStatus?
end

declare extern type EnumRtlTextSupport extends EnumItem with
end

declare extern type EnumRtlTextSupport_INTERNAL extends Enum with
	Default: EnumRtlTextSupport
	Disabled: EnumRtlTextSupport
	Enabled: EnumRtlTextSupport
	function GetEnumItems(self): { EnumRtlTextSupport }
	function FromName(self, name: string): EnumRtlTextSupport?
	function FromValue(self, value: number): EnumRtlTextSupport?
end

declare extern type EnumRunContext extends EnumItem with
end

declare extern type EnumRunContext_INTERNAL extends Enum with
	Legacy: EnumRunContext
	Server: EnumRunContext
	Client: EnumRunContext
	Plugin: EnumRunContext
	function GetEnumItems(self): { EnumRunContext }
	function FromName(self, name: string): EnumRunContext?
	function FromValue(self, value: number): EnumRunContext?
end

declare extern type EnumRunState extends EnumItem with
end

declare extern type EnumRunState_INTERNAL extends Enum with
	Stopped: EnumRunState
	Running: EnumRunState
	Paused: EnumRunState
	function GetEnumItems(self): { EnumRunState }
	function FromName(self, name: string): EnumRunState?
	function FromValue(self, value: number): EnumRunState?
end

declare extern type EnumRuntimeUndoBehavior extends EnumItem with
end

declare extern type EnumRuntimeUndoBehavior_INTERNAL extends Enum with
	Aggregate: EnumRuntimeUndoBehavior
	Snapshot: EnumRuntimeUndoBehavior
	Hybrid: EnumRuntimeUndoBehavior
	function GetEnumItems(self): { EnumRuntimeUndoBehavior }
	function FromName(self, name: string): EnumRuntimeUndoBehavior?
	function FromValue(self, value: number): EnumRuntimeUndoBehavior?
end

declare extern type EnumSafeAreaCompatibility extends EnumItem with
end

declare extern type EnumSafeAreaCompatibility_INTERNAL extends Enum with
	None: EnumSafeAreaCompatibility
	FullscreenExtension: EnumSafeAreaCompatibility
	function GetEnumItems(self): { EnumSafeAreaCompatibility }
	function FromName(self, name: string): EnumSafeAreaCompatibility?
	function FromValue(self, value: number): EnumSafeAreaCompatibility?
end

declare extern type EnumSalesTypeFilter extends EnumItem with
end

declare extern type EnumSalesTypeFilter_INTERNAL extends Enum with
	All: EnumSalesTypeFilter
	Collectibles: EnumSalesTypeFilter
	Premium: EnumSalesTypeFilter
	TimedOptions: EnumSalesTypeFilter
	function GetEnumItems(self): { EnumSalesTypeFilter }
	function FromName(self, name: string): EnumSalesTypeFilter?
	function FromValue(self, value: number): EnumSalesTypeFilter?
end

declare extern type EnumSandboxedInstanceMode extends EnumItem with
end

declare extern type EnumSandboxedInstanceMode_INTERNAL extends Enum with
	Default: EnumSandboxedInstanceMode
	Experimental: EnumSandboxedInstanceMode
	function GetEnumItems(self): { EnumSandboxedInstanceMode }
	function FromName(self, name: string): EnumSandboxedInstanceMode?
	function FromValue(self, value: number): EnumSandboxedInstanceMode?
end

declare extern type EnumSaveAvatarThumbnailCustomizationFailure extends EnumItem with
end

declare extern type EnumSaveAvatarThumbnailCustomizationFailure_INTERNAL extends Enum with
	BadThumbnailType: EnumSaveAvatarThumbnailCustomizationFailure
	BadYRotDeg: EnumSaveAvatarThumbnailCustomizationFailure
	BadFieldOfViewDeg: EnumSaveAvatarThumbnailCustomizationFailure
	BadDistanceScale: EnumSaveAvatarThumbnailCustomizationFailure
	Other: EnumSaveAvatarThumbnailCustomizationFailure
	Throttled: EnumSaveAvatarThumbnailCustomizationFailure
	function GetEnumItems(self): { EnumSaveAvatarThumbnailCustomizationFailure }
	function FromName(self, name: string): EnumSaveAvatarThumbnailCustomizationFailure?
	function FromValue(self, value: number): EnumSaveAvatarThumbnailCustomizationFailure?
end

declare extern type EnumSaveFilter extends EnumItem with
end

declare extern type EnumSaveFilter_INTERNAL extends Enum with
	SaveWorld: EnumSaveFilter
	SaveGame: EnumSaveFilter
	SaveAll: EnumSaveFilter
	function GetEnumItems(self): { EnumSaveFilter }
	function FromName(self, name: string): EnumSaveFilter?
	function FromValue(self, value: number): EnumSaveFilter?
end

declare extern type EnumSavedQualitySetting extends EnumItem with
end

declare extern type EnumSavedQualitySetting_INTERNAL extends Enum with
	Automatic: EnumSavedQualitySetting
	QualityLevel1: EnumSavedQualitySetting
	QualityLevel2: EnumSavedQualitySetting
	QualityLevel3: EnumSavedQualitySetting
	QualityLevel4: EnumSavedQualitySetting
	QualityLevel5: EnumSavedQualitySetting
	QualityLevel6: EnumSavedQualitySetting
	QualityLevel7: EnumSavedQualitySetting
	QualityLevel8: EnumSavedQualitySetting
	QualityLevel9: EnumSavedQualitySetting
	QualityLevel10: EnumSavedQualitySetting
	function GetEnumItems(self): { EnumSavedQualitySetting }
	function FromName(self, name: string): EnumSavedQualitySetting?
	function FromValue(self, value: number): EnumSavedQualitySetting?
end

declare extern type EnumScaleType extends EnumItem with
end

declare extern type EnumScaleType_INTERNAL extends Enum with
	Stretch: EnumScaleType
	Slice: EnumScaleType
	Tile: EnumScaleType
	Fit: EnumScaleType
	Crop: EnumScaleType
	function GetEnumItems(self): { EnumScaleType }
	function FromName(self, name: string): EnumScaleType?
	function FromValue(self, value: number): EnumScaleType?
end

declare extern type EnumScopeCheckResult extends EnumItem with
end

declare extern type EnumScopeCheckResult_INTERNAL extends Enum with
	ConsentAccepted: EnumScopeCheckResult
	InvalidScopes: EnumScopeCheckResult
	Timeout: EnumScopeCheckResult
	NoUserInput: EnumScopeCheckResult
	BackendError: EnumScopeCheckResult
	UnexpectedError: EnumScopeCheckResult
	InvalidArgument: EnumScopeCheckResult
	ConsentDenied: EnumScopeCheckResult
	function GetEnumItems(self): { EnumScopeCheckResult }
	function FromName(self, name: string): EnumScopeCheckResult?
	function FromValue(self, value: number): EnumScopeCheckResult?
end

declare extern type EnumScreenInsets extends EnumItem with
end

declare extern type EnumScreenInsets_INTERNAL extends Enum with
	None: EnumScreenInsets
	DeviceSafeInsets: EnumScreenInsets
	CoreUISafeInsets: EnumScreenInsets
	TopbarSafeInsets: EnumScreenInsets
	function GetEnumItems(self): { EnumScreenInsets }
	function FromName(self, name: string): EnumScreenInsets?
	function FromValue(self, value: number): EnumScreenInsets?
end

declare extern type EnumScreenOrientation extends EnumItem with
end

declare extern type EnumScreenOrientation_INTERNAL extends Enum with
	LandscapeLeft: EnumScreenOrientation
	LandscapeRight: EnumScreenOrientation
	LandscapeSensor: EnumScreenOrientation
	Portrait: EnumScreenOrientation
	Sensor: EnumScreenOrientation
	function GetEnumItems(self): { EnumScreenOrientation }
	function FromName(self, name: string): EnumScreenOrientation?
	function FromValue(self, value: number): EnumScreenOrientation?
end

declare extern type EnumScreenshotCaptureResult extends EnumItem with
end

declare extern type EnumScreenshotCaptureResult_INTERNAL extends Enum with
	Success: EnumScreenshotCaptureResult
	OtherError: EnumScreenshotCaptureResult
	NoDeviceSupport: EnumScreenshotCaptureResult
	NoSpaceOnDevice: EnumScreenshotCaptureResult
	function GetEnumItems(self): { EnumScreenshotCaptureResult }
	function FromName(self, name: string): EnumScreenshotCaptureResult?
	function FromValue(self, value: number): EnumScreenshotCaptureResult?
end

declare extern type EnumScriptStoppedReason extends EnumItem with
end

declare extern type EnumScriptStoppedReason_INTERNAL extends Enum with
	Breakpoint: EnumScriptStoppedReason
	Exception: EnumScriptStoppedReason
	Pause: EnumScriptStoppedReason
	Step: EnumScriptStoppedReason
	Entry: EnumScriptStoppedReason
	function GetEnumItems(self): { EnumScriptStoppedReason }
	function FromName(self, name: string): EnumScriptStoppedReason?
	function FromValue(self, value: number): EnumScriptStoppedReason?
end

declare extern type EnumScriptVariableScope extends EnumItem with
end

declare extern type EnumScriptVariableScope_INTERNAL extends Enum with
	Local: EnumScriptVariableScope
	Upvalue: EnumScriptVariableScope
	Global: EnumScriptVariableScope
	function GetEnumItems(self): { EnumScriptVariableScope }
	function FromName(self, name: string): EnumScriptVariableScope?
	function FromValue(self, value: number): EnumScriptVariableScope?
end

declare extern type EnumScrollBarInset extends EnumItem with
end

declare extern type EnumScrollBarInset_INTERNAL extends Enum with
	None: EnumScrollBarInset
	ScrollBar: EnumScrollBarInset
	Always: EnumScrollBarInset
	function GetEnumItems(self): { EnumScrollBarInset }
	function FromName(self, name: string): EnumScrollBarInset?
	function FromValue(self, value: number): EnumScrollBarInset?
end

declare extern type EnumScrollingDirection extends EnumItem with
end

declare extern type EnumScrollingDirection_INTERNAL extends Enum with
	X: EnumScrollingDirection
	Y: EnumScrollingDirection
	XY: EnumScrollingDirection
	function GetEnumItems(self): { EnumScrollingDirection }
	function FromName(self, name: string): EnumScrollingDirection?
	function FromValue(self, value: number): EnumScrollingDirection?
end

declare extern type EnumSecurityCapability extends EnumItem with
end

declare extern type EnumSecurityCapability_INTERNAL extends Enum with
	RunClientScript: EnumSecurityCapability
	RunServerScript: EnumSecurityCapability
	AccessOutsideWrite: EnumSecurityCapability
	AssetRequire: EnumSecurityCapability
	LoadString: EnumSecurityCapability
	ScriptGlobals: EnumSecurityCapability
	CreateInstances: EnumSecurityCapability
	Basic: EnumSecurityCapability
	Audio: EnumSecurityCapability
	DataStore: EnumSecurityCapability
	Network: EnumSecurityCapability
	Physics: EnumSecurityCapability
	UI: EnumSecurityCapability
	CSG: EnumSecurityCapability
	Chat: EnumSecurityCapability
	Animation: EnumSecurityCapability
	Avatar: EnumSecurityCapability
	Input: EnumSecurityCapability
	Environment: EnumSecurityCapability
	RemoteEvent: EnumSecurityCapability
	LegacySound: EnumSecurityCapability
	Players: EnumSecurityCapability
	CapabilityControl: EnumSecurityCapability
	Plugin: EnumSecurityCapability
	LocalUser: EnumSecurityCapability
	WritePlayer: EnumSecurityCapability
	RobloxScript: EnumSecurityCapability
	RobloxEngine: EnumSecurityCapability
	Unassigned: EnumSecurityCapability
	InternalTest: EnumSecurityCapability
	PluginOrOpenCloud: EnumSecurityCapability
	Assistant: EnumSecurityCapability
	RemoteCommand: EnumSecurityCapability
	AssetRead: EnumSecurityCapability
	AssetManagement: EnumSecurityCapability
	DynamicGeneration: EnumSecurityCapability
	PlatformAvatarEditing: EnumSecurityCapability
	AssetCreateUpdate: EnumSecurityCapability
	Capture: EnumSecurityCapability
	SensitiveInput: EnumSecurityCapability
	Monetization: EnumSecurityCapability
	LoadOwnedAsset: EnumSecurityCapability
	Social: EnumSecurityCapability
	ServerCommunication: EnumSecurityCapability
	Logging: EnumSecurityCapability
	PromptExternalPurchase: EnumSecurityCapability
	Groups: EnumSecurityCapability
	Teleport: EnumSecurityCapability
	Consequences: EnumSecurityCapability
	Material: EnumSecurityCapability
	AvatarBehavior: EnumSecurityCapability
	AvatarAppearance: EnumSecurityCapability
	LoadUnownedAsset: EnumSecurityCapability
	function GetEnumItems(self): { EnumSecurityCapability }
	function FromName(self, name: string): EnumSecurityCapability?
	function FromValue(self, value: number): EnumSecurityCapability?
end

declare extern type EnumSelectionBehavior extends EnumItem with
end

declare extern type EnumSelectionBehavior_INTERNAL extends Enum with
	Escape: EnumSelectionBehavior
	Stop: EnumSelectionBehavior
	function GetEnumItems(self): { EnumSelectionBehavior }
	function FromName(self, name: string): EnumSelectionBehavior?
	function FromValue(self, value: number): EnumSelectionBehavior?
end

declare extern type EnumSelectionRenderMode extends EnumItem with
end

declare extern type EnumSelectionRenderMode_INTERNAL extends Enum with
	Outlines: EnumSelectionRenderMode
	BoundingBoxes: EnumSelectionRenderMode
	Both: EnumSelectionRenderMode
	function GetEnumItems(self): { EnumSelectionRenderMode }
	function FromName(self, name: string): EnumSelectionRenderMode?
	function FromValue(self, value: number): EnumSelectionRenderMode?
end

declare extern type EnumSelfViewPosition extends EnumItem with
end

declare extern type EnumSelfViewPosition_INTERNAL extends Enum with
	LastPosition: EnumSelfViewPosition
	TopLeft: EnumSelfViewPosition
	TopRight: EnumSelfViewPosition
	BottomLeft: EnumSelfViewPosition
	BottomRight: EnumSelfViewPosition
	function GetEnumItems(self): { EnumSelfViewPosition }
	function FromName(self, name: string): EnumSelfViewPosition?
	function FromValue(self, value: number): EnumSelfViewPosition?
end

declare extern type EnumSensorMode extends EnumItem with
end

declare extern type EnumSensorMode_INTERNAL extends Enum with
	Floor: EnumSensorMode
	Ladder: EnumSensorMode
	ClassicFloor: EnumSensorMode
	ClassicLadder: EnumSensorMode
	function GetEnumItems(self): { EnumSensorMode }
	function FromName(self, name: string): EnumSensorMode?
	function FromValue(self, value: number): EnumSensorMode?
end

declare extern type EnumSensorUpdateType extends EnumItem with
end

declare extern type EnumSensorUpdateType_INTERNAL extends Enum with
	OnRead: EnumSensorUpdateType
	Manual: EnumSensorUpdateType
	function GetEnumItems(self): { EnumSensorUpdateType }
	function FromName(self, name: string): EnumSensorUpdateType?
	function FromValue(self, value: number): EnumSensorUpdateType?
end

declare extern type EnumServerLiveEditingMode extends EnumItem with
end

declare extern type EnumServerLiveEditingMode_INTERNAL extends Enum with
	Uninitialized: EnumServerLiveEditingMode
	Enabled: EnumServerLiveEditingMode
	Disabled: EnumServerLiveEditingMode
	function GetEnumItems(self): { EnumServerLiveEditingMode }
	function FromName(self, name: string): EnumServerLiveEditingMode?
	function FromValue(self, value: number): EnumServerLiveEditingMode?
end

declare extern type EnumServiceVisibility extends EnumItem with
end

declare extern type EnumServiceVisibility_INTERNAL extends Enum with
	Always: EnumServiceVisibility
	Off: EnumServiceVisibility
	WithChildren: EnumServiceVisibility
	function GetEnumItems(self): { EnumServiceVisibility }
	function FromName(self, name: string): EnumServiceVisibility?
	function FromValue(self, value: number): EnumServiceVisibility?
end

declare extern type EnumSeverity extends EnumItem with
end

declare extern type EnumSeverity_INTERNAL extends Enum with
	Error: EnumSeverity
	Warning: EnumSeverity
	Information: EnumSeverity
	Hint: EnumSeverity
	function GetEnumItems(self): { EnumSeverity }
	function FromName(self, name: string): EnumSeverity?
	function FromValue(self, value: number): EnumSeverity?
end

declare extern type EnumShowAdResult extends EnumItem with
end

declare extern type EnumShowAdResult_INTERNAL extends Enum with
	ShowCompleted: EnumShowAdResult
	AdNotReady: EnumShowAdResult
	AdAlreadyShowing: EnumShowAdResult
	InternalError: EnumShowAdResult
	ShowInterrupted: EnumShowAdResult
	InsufficientMemory: EnumShowAdResult
	function GetEnumItems(self): { EnumShowAdResult }
	function FromName(self, name: string): EnumShowAdResult?
	function FromValue(self, value: number): EnumShowAdResult?
end

declare extern type EnumSignalBehavior extends EnumItem with
end

declare extern type EnumSignalBehavior_INTERNAL extends Enum with
	Default: EnumSignalBehavior
	Immediate: EnumSignalBehavior
	Deferred: EnumSignalBehavior
	AncestryDeferred: EnumSignalBehavior
	function GetEnumItems(self): { EnumSignalBehavior }
	function FromName(self, name: string): EnumSignalBehavior?
	function FromValue(self, value: number): EnumSignalBehavior?
end

declare extern type EnumSimulationMode extends EnumItem with
end

declare extern type EnumSimulationMode_INTERNAL extends Enum with
	Default: EnumSimulationMode
	Enabled: EnumSimulationMode
	Disabled: EnumSimulationMode
	function GetEnumItems(self): { EnumSimulationMode }
	function FromName(self, name: string): EnumSimulationMode?
	function FromValue(self, value: number): EnumSimulationMode?
end

declare extern type EnumSizeConstraint extends EnumItem with
end

declare extern type EnumSizeConstraint_INTERNAL extends Enum with
	RelativeXY: EnumSizeConstraint
	RelativeXX: EnumSizeConstraint
	RelativeYY: EnumSizeConstraint
	function GetEnumItems(self): { EnumSizeConstraint }
	function FromName(self, name: string): EnumSizeConstraint?
	function FromValue(self, value: number): EnumSizeConstraint?
end

declare extern type EnumSlimTintMode extends EnumItem with
end

declare extern type EnumSlimTintMode_INTERNAL extends Enum with
	None: EnumSlimTintMode
	LOD: EnumSlimTintMode
	Meshes: EnumSlimTintMode
	MeshResourcePtr: EnumSlimTintMode
	ContentId: EnumSlimTintMode
	Zone: EnumSlimTintMode
	function GetEnumItems(self): { EnumSlimTintMode }
	function FromName(self, name: string): EnumSlimTintMode?
	function FromValue(self, value: number): EnumSlimTintMode?
end

declare extern type EnumSolidPrimitiveType extends EnumItem with
end

declare extern type EnumSolidPrimitiveType_INTERNAL extends Enum with
	Capsule: EnumSolidPrimitiveType
	Cone: EnumSolidPrimitiveType
	RoundedBox: EnumSolidPrimitiveType
	function GetEnumItems(self): { EnumSolidPrimitiveType }
	function FromName(self, name: string): EnumSolidPrimitiveType?
	function FromValue(self, value: number): EnumSolidPrimitiveType?
end

declare extern type EnumSolverConvergenceMetricType extends EnumItem with
end

declare extern type EnumSolverConvergenceMetricType_INTERNAL extends Enum with
	IterationBased: EnumSolverConvergenceMetricType
	AlgorithmAgnostic: EnumSolverConvergenceMetricType
	function GetEnumItems(self): { EnumSolverConvergenceMetricType }
	function FromName(self, name: string): EnumSolverConvergenceMetricType?
	function FromValue(self, value: number): EnumSolverConvergenceMetricType?
end

declare extern type EnumSolverConvergenceVisualizationMode extends EnumItem with
end

declare extern type EnumSolverConvergenceVisualizationMode_INTERNAL extends Enum with
	Disabled: EnumSolverConvergenceVisualizationMode
	PerIsland: EnumSolverConvergenceVisualizationMode
	PerEdge: EnumSolverConvergenceVisualizationMode
	function GetEnumItems(self): { EnumSolverConvergenceVisualizationMode }
	function FromName(self, name: string): EnumSolverConvergenceVisualizationMode?
	function FromValue(self, value: number): EnumSolverConvergenceVisualizationMode?
end

declare extern type EnumSortDirection extends EnumItem with
end

declare extern type EnumSortDirection_INTERNAL extends Enum with
	Ascending: EnumSortDirection
	Descending: EnumSortDirection
	function GetEnumItems(self): { EnumSortDirection }
	function FromName(self, name: string): EnumSortDirection?
	function FromValue(self, value: number): EnumSortDirection?
end

declare extern type EnumSortOrder extends EnumItem with
end

declare extern type EnumSortOrder_INTERNAL extends Enum with
	Name: EnumSortOrder
	Custom: EnumSortOrder
	LayoutOrder: EnumSortOrder
	function GetEnumItems(self): { EnumSortOrder }
	function FromName(self, name: string): EnumSortOrder?
	function FromValue(self, value: number): EnumSortOrder?
end

declare extern type EnumSpecialKey extends EnumItem with
end

declare extern type EnumSpecialKey_INTERNAL extends Enum with
	Insert: EnumSpecialKey
	Home: EnumSpecialKey
	End: EnumSpecialKey
	PageUp: EnumSpecialKey
	PageDown: EnumSpecialKey
	ChatHotkey: EnumSpecialKey
	function GetEnumItems(self): { EnumSpecialKey }
	function FromName(self, name: string): EnumSpecialKey?
	function FromValue(self, value: number): EnumSpecialKey?
end

declare extern type EnumStartCorner extends EnumItem with
end

declare extern type EnumStartCorner_INTERNAL extends Enum with
	TopLeft: EnumStartCorner
	TopRight: EnumStartCorner
	BottomLeft: EnumStartCorner
	BottomRight: EnumStartCorner
	function GetEnumItems(self): { EnumStartCorner }
	function FromName(self, name: string): EnumStartCorner?
	function FromValue(self, value: number): EnumStartCorner?
end

declare extern type EnumStateObjectFieldType extends EnumItem with
end

declare extern type EnumStateObjectFieldType_INTERNAL extends Enum with
	Boolean: EnumStateObjectFieldType
	CFrame: EnumStateObjectFieldType
	Color3: EnumStateObjectFieldType
	Float: EnumStateObjectFieldType
	Instance: EnumStateObjectFieldType
	Random: EnumStateObjectFieldType
	Vector2: EnumStateObjectFieldType
	Vector3: EnumStateObjectFieldType
	INVALID: EnumStateObjectFieldType
	function GetEnumItems(self): { EnumStateObjectFieldType }
	function FromName(self, name: string): EnumStateObjectFieldType?
	function FromValue(self, value: number): EnumStateObjectFieldType?
end

declare extern type EnumStatus extends EnumItem with
end

declare extern type EnumStatus_INTERNAL extends Enum with
	Poison: EnumStatus
	Confusion: EnumStatus
	function GetEnumItems(self): { EnumStatus }
	function FromName(self, name: string): EnumStatus?
	function FromValue(self, value: number): EnumStatus?
end

declare extern type EnumStepFrequency extends EnumItem with
end

declare extern type EnumStepFrequency_INTERNAL extends Enum with
	Hz60: EnumStepFrequency
	Hz30: EnumStepFrequency
	Hz15: EnumStepFrequency
	Hz10: EnumStepFrequency
	Hz5: EnumStepFrequency
	Hz1: EnumStepFrequency
	function GetEnumItems(self): { EnumStepFrequency }
	function FromName(self, name: string): EnumStepFrequency?
	function FromValue(self, value: number): EnumStepFrequency?
end

declare extern type EnumStreamOutBehavior extends EnumItem with
end

declare extern type EnumStreamOutBehavior_INTERNAL extends Enum with
	Default: EnumStreamOutBehavior
	LowMemory: EnumStreamOutBehavior
	Opportunistic: EnumStreamOutBehavior
	function GetEnumItems(self): { EnumStreamOutBehavior }
	function FromName(self, name: string): EnumStreamOutBehavior?
	function FromValue(self, value: number): EnumStreamOutBehavior?
end

declare extern type EnumStreamingIntegrityMode extends EnumItem with
end

declare extern type EnumStreamingIntegrityMode_INTERNAL extends Enum with
	Default: EnumStreamingIntegrityMode
	Disabled: EnumStreamingIntegrityMode
	MinimumRadiusPause: EnumStreamingIntegrityMode
	PauseOutsideLoadedArea: EnumStreamingIntegrityMode
	function GetEnumItems(self): { EnumStreamingIntegrityMode }
	function FromName(self, name: string): EnumStreamingIntegrityMode?
	function FromValue(self, value: number): EnumStreamingIntegrityMode?
end

declare extern type EnumStreamingPauseMode extends EnumItem with
end

declare extern type EnumStreamingPauseMode_INTERNAL extends Enum with
	Default: EnumStreamingPauseMode
	Disabled: EnumStreamingPauseMode
	ClientPhysicsPause: EnumStreamingPauseMode
	function GetEnumItems(self): { EnumStreamingPauseMode }
	function FromName(self, name: string): EnumStreamingPauseMode?
	function FromValue(self, value: number): EnumStreamingPauseMode?
end

declare extern type EnumStrokeSizingMode extends EnumItem with
end

declare extern type EnumStrokeSizingMode_INTERNAL extends Enum with
	FixedSize: EnumStrokeSizingMode
	ScaledSize: EnumStrokeSizingMode
	function GetEnumItems(self): { EnumStrokeSizingMode }
	function FromName(self, name: string): EnumStrokeSizingMode?
	function FromValue(self, value: number): EnumStrokeSizingMode?
end

declare extern type EnumStudioAction extends EnumItem with
end

declare extern type EnumStudioAction_INTERNAL extends Enum with
	Undo: EnumStudioAction
	Redo: EnumStudioAction
	SelectAll: EnumStudioAction
	DeleteSelected: EnumStudioAction
	DuplicateSelection: EnumStudioAction
	ClearSelection: EnumStudioAction
	ZoomExtents: EnumStudioAction
	Copy: EnumStudioAction
	Cut: EnumStudioAction
	Paste: EnumStudioAction
	function GetEnumItems(self): { EnumStudioAction }
	function FromName(self, name: string): EnumStudioAction?
	function FromValue(self, value: number): EnumStudioAction?
end

declare extern type EnumStudioCaptureBufferStatus extends EnumItem with
end

declare extern type EnumStudioCaptureBufferStatus_INTERNAL extends Enum with
	NotStarted: EnumStudioCaptureBufferStatus
	Pending: EnumStudioCaptureBufferStatus
	Ready: EnumStudioCaptureBufferStatus
	Error: EnumStudioCaptureBufferStatus
	function GetEnumItems(self): { EnumStudioCaptureBufferStatus }
	function FromName(self, name: string): EnumStudioCaptureBufferStatus?
	function FromValue(self, value: number): EnumStudioCaptureBufferStatus?
end

declare extern type EnumStudioCaptureScreenshotFormat extends EnumItem with
end

declare extern type EnumStudioCaptureScreenshotFormat_INTERNAL extends Enum with
	RGBA8: EnumStudioCaptureScreenshotFormat
	PNG: EnumStudioCaptureScreenshotFormat
	function GetEnumItems(self): { EnumStudioCaptureScreenshotFormat }
	function FromName(self, name: string): EnumStudioCaptureScreenshotFormat?
	function FromValue(self, value: number): EnumStudioCaptureScreenshotFormat?
end

declare extern type EnumStudioCloseMode extends EnumItem with
end

declare extern type EnumStudioCloseMode_INTERNAL extends Enum with
	None: EnumStudioCloseMode
	CloseStudio: EnumStudioCloseMode
	CloseDoc: EnumStudioCloseMode
	LogOut: EnumStudioCloseMode
	function GetEnumItems(self): { EnumStudioCloseMode }
	function FromName(self, name: string): EnumStudioCloseMode?
	function FromValue(self, value: number): EnumStudioCloseMode?
end

declare extern type EnumStudioDataModelType extends EnumItem with
end

declare extern type EnumStudioDataModelType_INTERNAL extends Enum with
	Edit: EnumStudioDataModelType
	PlayClient: EnumStudioDataModelType
	PlayServer: EnumStudioDataModelType
	Standalone: EnumStudioDataModelType
	None: EnumStudioDataModelType
	function GetEnumItems(self): { EnumStudioDataModelType }
	function FromName(self, name: string): EnumStudioDataModelType?
	function FromValue(self, value: number): EnumStudioDataModelType?
end

declare extern type EnumStudioPlaceUpdateFailureReason extends EnumItem with
end

declare extern type EnumStudioPlaceUpdateFailureReason_INTERNAL extends Enum with
	Other: EnumStudioPlaceUpdateFailureReason
	TeamCreateConflict: EnumStudioPlaceUpdateFailureReason
	function GetEnumItems(self): { EnumStudioPlaceUpdateFailureReason }
	function FromName(self, name: string): EnumStudioPlaceUpdateFailureReason?
	function FromValue(self, value: number): EnumStudioPlaceUpdateFailureReason?
end

declare extern type EnumStudioScriptEditorColorCategories extends EnumItem with
end

declare extern type EnumStudioScriptEditorColorCategories_INTERNAL extends Enum with
	Default: EnumStudioScriptEditorColorCategories
	Operator: EnumStudioScriptEditorColorCategories
	Number: EnumStudioScriptEditorColorCategories
	String: EnumStudioScriptEditorColorCategories
	Comment: EnumStudioScriptEditorColorCategories
	Keyword: EnumStudioScriptEditorColorCategories
	Builtin: EnumStudioScriptEditorColorCategories
	Method: EnumStudioScriptEditorColorCategories
	Property: EnumStudioScriptEditorColorCategories
	Nil: EnumStudioScriptEditorColorCategories
	Bool: EnumStudioScriptEditorColorCategories
	Function: EnumStudioScriptEditorColorCategories
	Local: EnumStudioScriptEditorColorCategories
	Self: EnumStudioScriptEditorColorCategories
	LuauKeyword: EnumStudioScriptEditorColorCategories
	FunctionName: EnumStudioScriptEditorColorCategories
	TODO: EnumStudioScriptEditorColorCategories
	Background: EnumStudioScriptEditorColorCategories
	SelectionText: EnumStudioScriptEditorColorCategories
	SelectionBackground: EnumStudioScriptEditorColorCategories
	FindSelectionBackground: EnumStudioScriptEditorColorCategories
	MatchingWordBackground: EnumStudioScriptEditorColorCategories
	Warning: EnumStudioScriptEditorColorCategories
	Error: EnumStudioScriptEditorColorCategories
	Info: EnumStudioScriptEditorColorCategories
	Hint: EnumStudioScriptEditorColorCategories
	Whitespace: EnumStudioScriptEditorColorCategories
	ActiveLine: EnumStudioScriptEditorColorCategories
	DebuggerCurrentLine: EnumStudioScriptEditorColorCategories
	DebuggerErrorLine: EnumStudioScriptEditorColorCategories
	Ruler: EnumStudioScriptEditorColorCategories
	Bracket: EnumStudioScriptEditorColorCategories
	Type: EnumStudioScriptEditorColorCategories
	MenuPrimaryText: EnumStudioScriptEditorColorCategories
	MenuSecondaryText: EnumStudioScriptEditorColorCategories
	MenuSelectedText: EnumStudioScriptEditorColorCategories
	MenuBackground: EnumStudioScriptEditorColorCategories
	MenuSelectedBackground: EnumStudioScriptEditorColorCategories
	MenuScrollbarBackground: EnumStudioScriptEditorColorCategories
	MenuScrollbarHandle: EnumStudioScriptEditorColorCategories
	MenuBorder: EnumStudioScriptEditorColorCategories
	DocViewCodeBackground: EnumStudioScriptEditorColorCategories
	AICOOverlayText: EnumStudioScriptEditorColorCategories
	AICOOverlayButtonBackground: EnumStudioScriptEditorColorCategories
	AICOOverlayButtonBackgroundHover: EnumStudioScriptEditorColorCategories
	AICOOverlayButtonBackgroundPressed: EnumStudioScriptEditorColorCategories
	IndentationRuler: EnumStudioScriptEditorColorCategories
	function GetEnumItems(self): { EnumStudioScriptEditorColorCategories }
	function FromName(self, name: string): EnumStudioScriptEditorColorCategories?
	function FromValue(self, value: number): EnumStudioScriptEditorColorCategories?
end

declare extern type EnumStudioScriptEditorColorPresets extends EnumItem with
end

declare extern type EnumStudioScriptEditorColorPresets_INTERNAL extends Enum with
	RobloxDefault: EnumStudioScriptEditorColorPresets
	Extra1: EnumStudioScriptEditorColorPresets
	Extra2: EnumStudioScriptEditorColorPresets
	Custom: EnumStudioScriptEditorColorPresets
	function GetEnumItems(self): { EnumStudioScriptEditorColorPresets }
	function FromName(self, name: string): EnumStudioScriptEditorColorPresets?
	function FromValue(self, value: number): EnumStudioScriptEditorColorPresets?
end

declare extern type EnumStudioStyleGuideColor extends EnumItem with
end

declare extern type EnumStudioStyleGuideColor_INTERNAL extends Enum with
	MainBackground: EnumStudioStyleGuideColor
	Titlebar: EnumStudioStyleGuideColor
	Dropdown: EnumStudioStyleGuideColor
	Tooltip: EnumStudioStyleGuideColor
	Notification: EnumStudioStyleGuideColor
	ScrollBar: EnumStudioStyleGuideColor
	ScrollBarBackground: EnumStudioStyleGuideColor
	TabBar: EnumStudioStyleGuideColor
	Tab: EnumStudioStyleGuideColor
	FilterButtonDefault: EnumStudioStyleGuideColor
	FilterButtonHover: EnumStudioStyleGuideColor
	FilterButtonChecked: EnumStudioStyleGuideColor
	FilterButtonAccent: EnumStudioStyleGuideColor
	FilterButtonBorder: EnumStudioStyleGuideColor
	FilterButtonBorderAlt: EnumStudioStyleGuideColor
	RibbonTab: EnumStudioStyleGuideColor
	RibbonTabTopBar: EnumStudioStyleGuideColor
	Button: EnumStudioStyleGuideColor
	MainButton: EnumStudioStyleGuideColor
	RibbonButton: EnumStudioStyleGuideColor
	ViewPortBackground: EnumStudioStyleGuideColor
	InputFieldBackground: EnumStudioStyleGuideColor
	Item: EnumStudioStyleGuideColor
	TableItem: EnumStudioStyleGuideColor
	CategoryItem: EnumStudioStyleGuideColor
	GameSettingsTableItem: EnumStudioStyleGuideColor
	GameSettingsTooltip: EnumStudioStyleGuideColor
	EmulatorBar: EnumStudioStyleGuideColor
	EmulatorDropDown: EnumStudioStyleGuideColor
	ColorPickerFrame: EnumStudioStyleGuideColor
	CurrentMarker: EnumStudioStyleGuideColor
	Border: EnumStudioStyleGuideColor
	DropShadow: EnumStudioStyleGuideColor
	Shadow: EnumStudioStyleGuideColor
	Light: EnumStudioStyleGuideColor
	Dark: EnumStudioStyleGuideColor
	Mid: EnumStudioStyleGuideColor
	MainText: EnumStudioStyleGuideColor
	SubText: EnumStudioStyleGuideColor
	TitlebarText: EnumStudioStyleGuideColor
	BrightText: EnumStudioStyleGuideColor
	DimmedText: EnumStudioStyleGuideColor
	LinkText: EnumStudioStyleGuideColor
	WarningText: EnumStudioStyleGuideColor
	ErrorText: EnumStudioStyleGuideColor
	InfoText: EnumStudioStyleGuideColor
	SensitiveText: EnumStudioStyleGuideColor
	ScriptSideWidget: EnumStudioStyleGuideColor
	ScriptBackground: EnumStudioStyleGuideColor
	ScriptText: EnumStudioStyleGuideColor
	ScriptSelectionText: EnumStudioStyleGuideColor
	ScriptSelectionBackground: EnumStudioStyleGuideColor
	ScriptFindSelectionBackground: EnumStudioStyleGuideColor
	ScriptMatchingWordSelectionBackground: EnumStudioStyleGuideColor
	ScriptOperator: EnumStudioStyleGuideColor
	ScriptNumber: EnumStudioStyleGuideColor
	ScriptString: EnumStudioStyleGuideColor
	ScriptComment: EnumStudioStyleGuideColor
	ScriptKeyword: EnumStudioStyleGuideColor
	ScriptBuiltInFunction: EnumStudioStyleGuideColor
	ScriptWarning: EnumStudioStyleGuideColor
	ScriptError: EnumStudioStyleGuideColor
	ScriptInformation: EnumStudioStyleGuideColor
	ScriptHint: EnumStudioStyleGuideColor
	ScriptWhitespace: EnumStudioStyleGuideColor
	ScriptRuler: EnumStudioStyleGuideColor
	DocViewCodeBackground: EnumStudioStyleGuideColor
	DebuggerCurrentLine: EnumStudioStyleGuideColor
	DebuggerErrorLine: EnumStudioStyleGuideColor
	DiffFilePathText: EnumStudioStyleGuideColor
	DiffTextHunkInfo: EnumStudioStyleGuideColor
	DiffTextNoChange: EnumStudioStyleGuideColor
	DiffTextAddition: EnumStudioStyleGuideColor
	DiffTextDeletion: EnumStudioStyleGuideColor
	DiffTextSeparatorBackground: EnumStudioStyleGuideColor
	DiffTextNoChangeBackground: EnumStudioStyleGuideColor
	DiffTextAdditionBackground: EnumStudioStyleGuideColor
	DiffTextDeletionBackground: EnumStudioStyleGuideColor
	DiffLineNum: EnumStudioStyleGuideColor
	DiffLineNumSeparatorBackground: EnumStudioStyleGuideColor
	DiffLineNumNoChangeBackground: EnumStudioStyleGuideColor
	DiffLineNumAdditionBackground: EnumStudioStyleGuideColor
	DiffLineNumDeletionBackground: EnumStudioStyleGuideColor
	DiffFilePathBackground: EnumStudioStyleGuideColor
	DiffFilePathBorder: EnumStudioStyleGuideColor
	ChatIncomingBgColor: EnumStudioStyleGuideColor
	ChatIncomingTextColor: EnumStudioStyleGuideColor
	ChatOutgoingBgColor: EnumStudioStyleGuideColor
	ChatOutgoingTextColor: EnumStudioStyleGuideColor
	ChatModeratedMessageColor: EnumStudioStyleGuideColor
	Separator: EnumStudioStyleGuideColor
	ButtonBorder: EnumStudioStyleGuideColor
	ButtonText: EnumStudioStyleGuideColor
	InputFieldBorder: EnumStudioStyleGuideColor
	CheckedFieldBackground: EnumStudioStyleGuideColor
	CheckedFieldBorder: EnumStudioStyleGuideColor
	CheckedFieldIndicator: EnumStudioStyleGuideColor
	HeaderSection: EnumStudioStyleGuideColor
	Midlight: EnumStudioStyleGuideColor
	StatusBar: EnumStudioStyleGuideColor
	DialogButton: EnumStudioStyleGuideColor
	DialogButtonText: EnumStudioStyleGuideColor
	DialogButtonBorder: EnumStudioStyleGuideColor
	DialogMainButton: EnumStudioStyleGuideColor
	DialogMainButtonText: EnumStudioStyleGuideColor
	InfoBarWarningBackground: EnumStudioStyleGuideColor
	InfoBarWarningText: EnumStudioStyleGuideColor
	ScriptEditorCurrentLine: EnumStudioStyleGuideColor
	ScriptMethod: EnumStudioStyleGuideColor
	ScriptProperty: EnumStudioStyleGuideColor
	ScriptNil: EnumStudioStyleGuideColor
	ScriptBool: EnumStudioStyleGuideColor
	ScriptFunction: EnumStudioStyleGuideColor
	ScriptLocal: EnumStudioStyleGuideColor
	ScriptSelf: EnumStudioStyleGuideColor
	ScriptLuauKeyword: EnumStudioStyleGuideColor
	ScriptFunctionName: EnumStudioStyleGuideColor
	ScriptTodo: EnumStudioStyleGuideColor
	ScriptBracket: EnumStudioStyleGuideColor
	AttributeCog: EnumStudioStyleGuideColor
	AICOOverlayText: EnumStudioStyleGuideColor
	AICOOverlayButtonBackground: EnumStudioStyleGuideColor
	AICOOverlayButtonBackgroundHover: EnumStudioStyleGuideColor
	AICOOverlayButtonBackgroundPressed: EnumStudioStyleGuideColor
	OnboardingCover: EnumStudioStyleGuideColor
	OnboardingHighlight: EnumStudioStyleGuideColor
	OnboardingShadow: EnumStudioStyleGuideColor
	BreakpointMarker: EnumStudioStyleGuideColor
	DiffLineNumHover: EnumStudioStyleGuideColor
	DiffLineNumSeparatorBackgroundHover: EnumStudioStyleGuideColor
	function GetEnumItems(self): { EnumStudioStyleGuideColor }
	function FromName(self, name: string): EnumStudioStyleGuideColor?
	function FromValue(self, value: number): EnumStudioStyleGuideColor?
end

declare extern type EnumStudioStyleGuideModifier extends EnumItem with
end

declare extern type EnumStudioStyleGuideModifier_INTERNAL extends Enum with
	Default: EnumStudioStyleGuideModifier
	Selected: EnumStudioStyleGuideModifier
	Pressed: EnumStudioStyleGuideModifier
	Disabled: EnumStudioStyleGuideModifier
	Hover: EnumStudioStyleGuideModifier
	function GetEnumItems(self): { EnumStudioStyleGuideModifier }
	function FromName(self, name: string): EnumStudioStyleGuideModifier?
	function FromValue(self, value: number): EnumStudioStyleGuideModifier?
end

declare extern type EnumStyle extends EnumItem with
end

declare extern type EnumStyle_INTERNAL extends Enum with
	AlternatingSupports: EnumStyle
	BridgeStyleSupports: EnumStyle
	NoSupports: EnumStyle
	function GetEnumItems(self): { EnumStyle }
	function FromName(self, name: string): EnumStyle?
	function FromValue(self, value: number): EnumStyle?
end

declare extern type EnumSubscriptionExpirationReason extends EnumItem with
end

declare extern type EnumSubscriptionExpirationReason_INTERNAL extends Enum with
	ProductInactive: EnumSubscriptionExpirationReason
	ProductDeleted: EnumSubscriptionExpirationReason
	SubscriberCancelled: EnumSubscriptionExpirationReason
	SubscriberRefunded: EnumSubscriptionExpirationReason
	Lapsed: EnumSubscriptionExpirationReason
	function GetEnumItems(self): { EnumSubscriptionExpirationReason }
	function FromName(self, name: string): EnumSubscriptionExpirationReason?
	function FromValue(self, value: number): EnumSubscriptionExpirationReason?
end

declare extern type EnumSubscriptionPaymentStatus extends EnumItem with
end

declare extern type EnumSubscriptionPaymentStatus_INTERNAL extends Enum with
	Paid: EnumSubscriptionPaymentStatus
	Refunded: EnumSubscriptionPaymentStatus
	function GetEnumItems(self): { EnumSubscriptionPaymentStatus }
	function FromName(self, name: string): EnumSubscriptionPaymentStatus?
	function FromValue(self, value: number): EnumSubscriptionPaymentStatus?
end

declare extern type EnumSubscriptionPeriod extends EnumItem with
end

declare extern type EnumSubscriptionPeriod_INTERNAL extends Enum with
	Month: EnumSubscriptionPeriod
	function GetEnumItems(self): { EnumSubscriptionPeriod }
	function FromName(self, name: string): EnumSubscriptionPeriod?
	function FromValue(self, value: number): EnumSubscriptionPeriod?
end

declare extern type EnumSubscriptionState extends EnumItem with
end

declare extern type EnumSubscriptionState_INTERNAL extends Enum with
	NeverSubscribed: EnumSubscriptionState
	SubscribedWillRenew: EnumSubscriptionState
	SubscribedWillNotRenew: EnumSubscriptionState
	SubscribedRenewalPaymentPending: EnumSubscriptionState
	Expired: EnumSubscriptionState
	function GetEnumItems(self): { EnumSubscriptionState }
	function FromName(self, name: string): EnumSubscriptionState?
	function FromValue(self, value: number): EnumSubscriptionState?
end

declare extern type EnumSurfaceConstraint extends EnumItem with
end

declare extern type EnumSurfaceConstraint_INTERNAL extends Enum with
	None: EnumSurfaceConstraint
	Hinge: EnumSurfaceConstraint
	SteppingMotor: EnumSurfaceConstraint
	Motor: EnumSurfaceConstraint
	function GetEnumItems(self): { EnumSurfaceConstraint }
	function FromName(self, name: string): EnumSurfaceConstraint?
	function FromValue(self, value: number): EnumSurfaceConstraint?
end

declare extern type EnumSurfaceGuiShape extends EnumItem with
end

declare extern type EnumSurfaceGuiShape_INTERNAL extends Enum with
	Flat: EnumSurfaceGuiShape
	CurvedHorizontally: EnumSurfaceGuiShape
	function GetEnumItems(self): { EnumSurfaceGuiShape }
	function FromName(self, name: string): EnumSurfaceGuiShape?
	function FromValue(self, value: number): EnumSurfaceGuiShape?
end

declare extern type EnumSurfaceGuiSizingMode extends EnumItem with
end

declare extern type EnumSurfaceGuiSizingMode_INTERNAL extends Enum with
	FixedSize: EnumSurfaceGuiSizingMode
	PixelsPerStud: EnumSurfaceGuiSizingMode
	function GetEnumItems(self): { EnumSurfaceGuiSizingMode }
	function FromName(self, name: string): EnumSurfaceGuiSizingMode?
	function FromValue(self, value: number): EnumSurfaceGuiSizingMode?
end

declare extern type EnumSurfaceType extends EnumItem with
end

declare extern type EnumSurfaceType_INTERNAL extends Enum with
	Smooth: EnumSurfaceType
	Glue: EnumSurfaceType
	Weld: EnumSurfaceType
	Studs: EnumSurfaceType
	Inlet: EnumSurfaceType
	Universal: EnumSurfaceType
	Hinge: EnumSurfaceType
	Motor: EnumSurfaceType
	SteppingMotor: EnumSurfaceType
	SmoothNoOutlines: EnumSurfaceType
	function GetEnumItems(self): { EnumSurfaceType }
	function FromName(self, name: string): EnumSurfaceType?
	function FromValue(self, value: number): EnumSurfaceType?
end

declare extern type EnumSwipeDirection extends EnumItem with
end

declare extern type EnumSwipeDirection_INTERNAL extends Enum with
	Right: EnumSwipeDirection
	Left: EnumSwipeDirection
	Up: EnumSwipeDirection
	Down: EnumSwipeDirection
	None: EnumSwipeDirection
	function GetEnumItems(self): { EnumSwipeDirection }
	function FromName(self, name: string): EnumSwipeDirection?
	function FromValue(self, value: number): EnumSwipeDirection?
end

declare extern type EnumSystemThemeValue extends EnumItem with
end

declare extern type EnumSystemThemeValue_INTERNAL extends Enum with
	error: EnumSystemThemeValue
	light: EnumSystemThemeValue
	dark: EnumSystemThemeValue
	systemLight: EnumSystemThemeValue
	systemDark: EnumSystemThemeValue
	function GetEnumItems(self): { EnumSystemThemeValue }
	function FromName(self, name: string): EnumSystemThemeValue?
	function FromValue(self, value: number): EnumSystemThemeValue?
end

declare extern type EnumTableMajorAxis extends EnumItem with
end

declare extern type EnumTableMajorAxis_INTERNAL extends Enum with
	RowMajor: EnumTableMajorAxis
	ColumnMajor: EnumTableMajorAxis
	function GetEnumItems(self): { EnumTableMajorAxis }
	function FromName(self, name: string): EnumTableMajorAxis?
	function FromValue(self, value: number): EnumTableMajorAxis?
end

declare extern type EnumTagReplicability extends EnumItem with
end

declare extern type EnumTagReplicability_INTERNAL extends Enum with
	function GetEnumItems(self): { EnumTagReplicability }
	function FromName(self, name: string): EnumTagReplicability?
	function FromValue(self, value: number): EnumTagReplicability?
end

declare extern type EnumTeamCreateErrorState extends EnumItem with
end

declare extern type EnumTeamCreateErrorState_INTERNAL extends Enum with
	PlaceSizeTooLarge: EnumTeamCreateErrorState
	PlaceSizeApproachingLimit: EnumTeamCreateErrorState
	PlaceUploadFailing: EnumTeamCreateErrorState
	NoError: EnumTeamCreateErrorState
	function GetEnumItems(self): { EnumTeamCreateErrorState }
	function FromName(self, name: string): EnumTeamCreateErrorState?
	function FromValue(self, value: number): EnumTeamCreateErrorState?
end

declare extern type EnumTechnology extends EnumItem with
end

declare extern type EnumTechnology_INTERNAL extends Enum with
	Legacy: EnumTechnology
	Voxel: EnumTechnology
	Compatibility: EnumTechnology
	ShadowMap: EnumTechnology
	Future: EnumTechnology
	Unified: EnumTechnology
	function GetEnumItems(self): { EnumTechnology }
	function FromName(self, name: string): EnumTechnology?
	function FromValue(self, value: number): EnumTechnology?
end

declare extern type EnumTelemetryBackend extends EnumItem with
end

declare extern type EnumTelemetryBackend_INTERNAL extends Enum with
	UNSPECIFIED: EnumTelemetryBackend
	EventIngest: EnumTelemetryBackend
	Points: EnumTelemetryBackend
	Teletune: EnumTelemetryBackend
	EphemeralCounter: EnumTelemetryBackend
	EphemeralStat: EnumTelemetryBackend
	Counter: EnumTelemetryBackend
	Stat: EnumTelemetryBackend
	function GetEnumItems(self): { EnumTelemetryBackend }
	function FromName(self, name: string): EnumTelemetryBackend?
	function FromValue(self, value: number): EnumTelemetryBackend?
end

declare extern type EnumTelemetryStandardizedField extends EnumItem with
end

declare extern type EnumTelemetryStandardizedField_INTERNAL extends Enum with
	AddDatacenterId: EnumTelemetryStandardizedField
	AddPlaceId: EnumTelemetryStandardizedField
	AddUniverseId: EnumTelemetryStandardizedField
	AddPlaceInstanceId: EnumTelemetryStandardizedField
	AddPlaySessionId: EnumTelemetryStandardizedField
	AddCurrentContextName: EnumTelemetryStandardizedField
	AddOsInfo: EnumTelemetryStandardizedField
	AddArchitectureInfo: EnumTelemetryStandardizedField
	AddCpuInfo: EnumTelemetryStandardizedField
	AddMemoryInfo: EnumTelemetryStandardizedField
	AddSessionInfo: EnumTelemetryStandardizedField
	function GetEnumItems(self): { EnumTelemetryStandardizedField }
	function FromName(self, name: string): EnumTelemetryStandardizedField?
	function FromValue(self, value: number): EnumTelemetryStandardizedField?
end

declare extern type EnumTeleportMethod extends EnumItem with
end

declare extern type EnumTeleportMethod_INTERNAL extends Enum with
	TeleportToSpawnByName: EnumTeleportMethod
	TeleportToPlaceInstance: EnumTeleportMethod
	TeleportToPrivateServer: EnumTeleportMethod
	TeleportPartyAsync: EnumTeleportMethod
	TeleportToVIPServer: EnumTeleportMethod
	TeleportToInstanceBack: EnumTeleportMethod
	TeleportUnknown: EnumTeleportMethod
	function GetEnumItems(self): { EnumTeleportMethod }
	function FromName(self, name: string): EnumTeleportMethod?
	function FromValue(self, value: number): EnumTeleportMethod?
end

declare extern type EnumTeleportResult extends EnumItem with
end

declare extern type EnumTeleportResult_INTERNAL extends Enum with
	Success: EnumTeleportResult
	Failure: EnumTeleportResult
	GameNotFound: EnumTeleportResult
	GameEnded: EnumTeleportResult
	GameFull: EnumTeleportResult
	Unauthorized: EnumTeleportResult
	Flooded: EnumTeleportResult
	IsTeleporting: EnumTeleportResult
	function GetEnumItems(self): { EnumTeleportResult }
	function FromName(self, name: string): EnumTeleportResult?
	function FromValue(self, value: number): EnumTeleportResult?
end

declare extern type EnumTeleportState extends EnumItem with
end

declare extern type EnumTeleportState_INTERNAL extends Enum with
	RequestedFromServer: EnumTeleportState
	Started: EnumTeleportState
	WaitingForServer: EnumTeleportState
	Failed: EnumTeleportState
	InProgress: EnumTeleportState
	function GetEnumItems(self): { EnumTeleportState }
	function FromName(self, name: string): EnumTeleportState?
	function FromValue(self, value: number): EnumTeleportState?
end

declare extern type EnumTeleportType extends EnumItem with
end

declare extern type EnumTeleportType_INTERNAL extends Enum with
	ToPlace: EnumTeleportType
	ToInstance: EnumTeleportType
	ToReservedServer: EnumTeleportType
	ToVIPServer: EnumTeleportType
	ToInstanceBack: EnumTeleportType
	function GetEnumItems(self): { EnumTeleportType }
	function FromName(self, name: string): EnumTeleportType?
	function FromValue(self, value: number): EnumTeleportType?
end

declare extern type EnumTerrainAcquisitionMethod extends EnumItem with
end

declare extern type EnumTerrainAcquisitionMethod_INTERNAL extends Enum with
	None: EnumTerrainAcquisitionMethod
	Legacy: EnumTerrainAcquisitionMethod
	Template: EnumTerrainAcquisitionMethod
	Generate: EnumTerrainAcquisitionMethod
	Import: EnumTerrainAcquisitionMethod
	Convert: EnumTerrainAcquisitionMethod
	EditAddTool: EnumTerrainAcquisitionMethod
	EditSeaLevelTool: EnumTerrainAcquisitionMethod
	EditReplaceTool: EnumTerrainAcquisitionMethod
	RegionFillTool: EnumTerrainAcquisitionMethod
	RegionPasteTool: EnumTerrainAcquisitionMethod
	Other: EnumTerrainAcquisitionMethod
	function GetEnumItems(self): { EnumTerrainAcquisitionMethod }
	function FromName(self, name: string): EnumTerrainAcquisitionMethod?
	function FromValue(self, value: number): EnumTerrainAcquisitionMethod?
end

declare extern type EnumTerrainFace extends EnumItem with
end

declare extern type EnumTerrainFace_INTERNAL extends Enum with
	Top: EnumTerrainFace
	Side: EnumTerrainFace
	Bottom: EnumTerrainFace
	function GetEnumItems(self): { EnumTerrainFace }
	function FromName(self, name: string): EnumTerrainFace?
	function FromValue(self, value: number): EnumTerrainFace?
end

declare extern type EnumTerrainLiquidMergeOperation extends EnumItem with
end

declare extern type EnumTerrainLiquidMergeOperation_INTERNAL extends Enum with
	None: EnumTerrainLiquidMergeOperation
	Source: EnumTerrainLiquidMergeOperation
	Union: EnumTerrainLiquidMergeOperation
	Difference: EnumTerrainLiquidMergeOperation
	Intersect: EnumTerrainLiquidMergeOperation
	function GetEnumItems(self): { EnumTerrainLiquidMergeOperation }
	function FromName(self, name: string): EnumTerrainLiquidMergeOperation?
	function FromValue(self, value: number): EnumTerrainLiquidMergeOperation?
end

declare extern type EnumTerrainSolidMergeOperation extends EnumItem with
end

declare extern type EnumTerrainSolidMergeOperation_INTERNAL extends Enum with
	None: EnumTerrainSolidMergeOperation
	Paint: EnumTerrainSolidMergeOperation
	Source: EnumTerrainSolidMergeOperation
	Union: EnumTerrainSolidMergeOperation
	Dig: EnumTerrainSolidMergeOperation
	Difference: EnumTerrainSolidMergeOperation
	Intersect: EnumTerrainSolidMergeOperation
	Cut: EnumTerrainSolidMergeOperation
	Place: EnumTerrainSolidMergeOperation
	function GetEnumItems(self): { EnumTerrainSolidMergeOperation }
	function FromName(self, name: string): EnumTerrainSolidMergeOperation?
	function FromValue(self, value: number): EnumTerrainSolidMergeOperation?
end

declare extern type EnumTextChatMessageStatus extends EnumItem with
end

declare extern type EnumTextChatMessageStatus_INTERNAL extends Enum with
	Unknown: EnumTextChatMessageStatus
	Success: EnumTextChatMessageStatus
	Sending: EnumTextChatMessageStatus
	TextFilterFailed: EnumTextChatMessageStatus
	Floodchecked: EnumTextChatMessageStatus
	InvalidPrivacySettings: EnumTextChatMessageStatus
	InvalidTextChannelPermissions: EnumTextChatMessageStatus
	MessageTooLong: EnumTextChatMessageStatus
	ModerationTimeout: EnumTextChatMessageStatus
	function GetEnumItems(self): { EnumTextChatMessageStatus }
	function FromName(self, name: string): EnumTextChatMessageStatus?
	function FromValue(self, value: number): EnumTextChatMessageStatus?
end

declare extern type EnumTextDirection extends EnumItem with
end

declare extern type EnumTextDirection_INTERNAL extends Enum with
	Auto: EnumTextDirection
	LeftToRight: EnumTextDirection
	RightToLeft: EnumTextDirection
	function GetEnumItems(self): { EnumTextDirection }
	function FromName(self, name: string): EnumTextDirection?
	function FromValue(self, value: number): EnumTextDirection?
end

declare extern type EnumTextFilterContext extends EnumItem with
end

declare extern type EnumTextFilterContext_INTERNAL extends Enum with
	PublicChat: EnumTextFilterContext
	PrivateChat: EnumTextFilterContext
	function GetEnumItems(self): { EnumTextFilterContext }
	function FromName(self, name: string): EnumTextFilterContext?
	function FromValue(self, value: number): EnumTextFilterContext?
end

declare extern type EnumTextInputType extends EnumItem with
end

declare extern type EnumTextInputType_INTERNAL extends Enum with
	Default: EnumTextInputType
	NoSuggestions: EnumTextInputType
	Number: EnumTextInputType
	Email: EnumTextInputType
	Phone: EnumTextInputType
	Password: EnumTextInputType
	PasswordShown: EnumTextInputType
	Username: EnumTextInputType
	OneTimePassword: EnumTextInputType
	NewPassword: EnumTextInputType
	NewPasswordShown: EnumTextInputType
	function GetEnumItems(self): { EnumTextInputType }
	function FromName(self, name: string): EnumTextInputType?
	function FromValue(self, value: number): EnumTextInputType?
end

declare extern type EnumTextTruncate extends EnumItem with
end

declare extern type EnumTextTruncate_INTERNAL extends Enum with
	None: EnumTextTruncate
	AtEnd: EnumTextTruncate
	SplitWord: EnumTextTruncate
	function GetEnumItems(self): { EnumTextTruncate }
	function FromName(self, name: string): EnumTextTruncate?
	function FromValue(self, value: number): EnumTextTruncate?
end

declare extern type EnumTextXAlignment extends EnumItem with
end

declare extern type EnumTextXAlignment_INTERNAL extends Enum with
	Left: EnumTextXAlignment
	Right: EnumTextXAlignment
	Center: EnumTextXAlignment
	function GetEnumItems(self): { EnumTextXAlignment }
	function FromName(self, name: string): EnumTextXAlignment?
	function FromValue(self, value: number): EnumTextXAlignment?
end

declare extern type EnumTextYAlignment extends EnumItem with
end

declare extern type EnumTextYAlignment_INTERNAL extends Enum with
	Top: EnumTextYAlignment
	Center: EnumTextYAlignment
	Bottom: EnumTextYAlignment
	function GetEnumItems(self): { EnumTextYAlignment }
	function FromName(self, name: string): EnumTextYAlignment?
	function FromValue(self, value: number): EnumTextYAlignment?
end

declare extern type EnumTextureMode extends EnumItem with
end

declare extern type EnumTextureMode_INTERNAL extends Enum with
	Stretch: EnumTextureMode
	Wrap: EnumTextureMode
	Static: EnumTextureMode
	function GetEnumItems(self): { EnumTextureMode }
	function FromName(self, name: string): EnumTextureMode?
	function FromValue(self, value: number): EnumTextureMode?
end

declare extern type EnumTextureQueryType extends EnumItem with
end

declare extern type EnumTextureQueryType_INTERNAL extends Enum with
	NonHumanoid: EnumTextureQueryType
	NonHumanoidOrphaned: EnumTextureQueryType
	Humanoid: EnumTextureQueryType
	HumanoidOrphaned: EnumTextureQueryType
	function GetEnumItems(self): { EnumTextureQueryType }
	function FromName(self, name: string): EnumTextureQueryType?
	function FromValue(self, value: number): EnumTextureQueryType?
end

declare extern type EnumThreadPoolConfig extends EnumItem with
end

declare extern type EnumThreadPoolConfig_INTERNAL extends Enum with
	Auto: EnumThreadPoolConfig
	Threads1: EnumThreadPoolConfig
	Threads2: EnumThreadPoolConfig
	Threads3: EnumThreadPoolConfig
	Threads4: EnumThreadPoolConfig
	Threads8: EnumThreadPoolConfig
	Threads16: EnumThreadPoolConfig
	PerCore1: EnumThreadPoolConfig
	PerCore2: EnumThreadPoolConfig
	PerCore3: EnumThreadPoolConfig
	PerCore4: EnumThreadPoolConfig
	function GetEnumItems(self): { EnumThreadPoolConfig }
	function FromName(self, name: string): EnumThreadPoolConfig?
	function FromValue(self, value: number): EnumThreadPoolConfig?
end

declare extern type EnumThrottlingPriority extends EnumItem with
end

declare extern type EnumThrottlingPriority_INTERNAL extends Enum with
	Default: EnumThrottlingPriority
	ElevatedOnServer: EnumThrottlingPriority
	Extreme: EnumThrottlingPriority
	function GetEnumItems(self): { EnumThrottlingPriority }
	function FromName(self, name: string): EnumThrottlingPriority?
	function FromValue(self, value: number): EnumThrottlingPriority?
end

declare extern type EnumThumbnailSize extends EnumItem with
end

declare extern type EnumThumbnailSize_INTERNAL extends Enum with
	Size48x48: EnumThumbnailSize
	Size180x180: EnumThumbnailSize
	Size420x420: EnumThumbnailSize
	Size60x60: EnumThumbnailSize
	Size100x100: EnumThumbnailSize
	Size150x150: EnumThumbnailSize
	Size352x352: EnumThumbnailSize
	function GetEnumItems(self): { EnumThumbnailSize }
	function FromName(self, name: string): EnumThumbnailSize?
	function FromValue(self, value: number): EnumThumbnailSize?
end

declare extern type EnumThumbnailType extends EnumItem with
end

declare extern type EnumThumbnailType_INTERNAL extends Enum with
	HeadShot: EnumThumbnailType
	AvatarBust: EnumThumbnailType
	AvatarThumbnail: EnumThumbnailType
	function GetEnumItems(self): { EnumThumbnailType }
	function FromName(self, name: string): EnumThumbnailType?
	function FromValue(self, value: number): EnumThumbnailType?
end

declare extern type EnumTickCountSampleMethod extends EnumItem with
end

declare extern type EnumTickCountSampleMethod_INTERNAL extends Enum with
	Fast: EnumTickCountSampleMethod
	Benchmark: EnumTickCountSampleMethod
	Precise: EnumTickCountSampleMethod
	function GetEnumItems(self): { EnumTickCountSampleMethod }
	function FromName(self, name: string): EnumTickCountSampleMethod?
	function FromValue(self, value: number): EnumTickCountSampleMethod?
end

declare extern type EnumTonemapperPreset extends EnumItem with
end

declare extern type EnumTonemapperPreset_INTERNAL extends Enum with
	Default: EnumTonemapperPreset
	Retro: EnumTonemapperPreset
	function GetEnumItems(self): { EnumTonemapperPreset }
	function FromName(self, name: string): EnumTonemapperPreset?
	function FromValue(self, value: number): EnumTonemapperPreset?
end

declare extern type EnumTopBottom extends EnumItem with
end

declare extern type EnumTopBottom_INTERNAL extends Enum with
	Top: EnumTopBottom
	Center: EnumTopBottom
	Bottom: EnumTopBottom
	function GetEnumItems(self): { EnumTopBottom }
	function FromName(self, name: string): EnumTopBottom?
	function FromValue(self, value: number): EnumTopBottom?
end

declare extern type EnumTouchCameraMovementMode extends EnumItem with
end

declare extern type EnumTouchCameraMovementMode_INTERNAL extends Enum with
	Default: EnumTouchCameraMovementMode
	Classic: EnumTouchCameraMovementMode
	Follow: EnumTouchCameraMovementMode
	Orbital: EnumTouchCameraMovementMode
	function GetEnumItems(self): { EnumTouchCameraMovementMode }
	function FromName(self, name: string): EnumTouchCameraMovementMode?
	function FromValue(self, value: number): EnumTouchCameraMovementMode?
end

declare extern type EnumTouchMovementMode extends EnumItem with
end

declare extern type EnumTouchMovementMode_INTERNAL extends Enum with
	Default: EnumTouchMovementMode
	Thumbstick: EnumTouchMovementMode
	DPad: EnumTouchMovementMode
	Thumbpad: EnumTouchMovementMode
	ClickToMove: EnumTouchMovementMode
	DynamicThumbstick: EnumTouchMovementMode
	function GetEnumItems(self): { EnumTouchMovementMode }
	function FromName(self, name: string): EnumTouchMovementMode?
	function FromValue(self, value: number): EnumTouchMovementMode?
end

declare extern type EnumTrackerError extends EnumItem with
end

declare extern type EnumTrackerError_INTERNAL extends Enum with
	Ok: EnumTrackerError
	NoService: EnumTrackerError
	InitFailed: EnumTrackerError
	NoVideo: EnumTrackerError
	VideoError: EnumTrackerError
	VideoNoPermission: EnumTrackerError
	VideoUnsupported: EnumTrackerError
	NoAudio: EnumTrackerError
	AudioError: EnumTrackerError
	AudioNoPermission: EnumTrackerError
	UnsupportedDevice: EnumTrackerError
	function GetEnumItems(self): { EnumTrackerError }
	function FromName(self, name: string): EnumTrackerError?
	function FromValue(self, value: number): EnumTrackerError?
end

declare extern type EnumTrackerExtrapolationFlagMode extends EnumItem with
end

declare extern type EnumTrackerExtrapolationFlagMode_INTERNAL extends Enum with
	ForceDisabled: EnumTrackerExtrapolationFlagMode
	ExtrapolateFacsAndPose: EnumTrackerExtrapolationFlagMode
	ExtrapolateFacsOnly: EnumTrackerExtrapolationFlagMode
	Auto: EnumTrackerExtrapolationFlagMode
	function GetEnumItems(self): { EnumTrackerExtrapolationFlagMode }
	function FromName(self, name: string): EnumTrackerExtrapolationFlagMode?
	function FromValue(self, value: number): EnumTrackerExtrapolationFlagMode?
end

declare extern type EnumTrackerFaceTrackingStatus extends EnumItem with
end

declare extern type EnumTrackerFaceTrackingStatus_INTERNAL extends Enum with
	FaceTrackingSuccess: EnumTrackerFaceTrackingStatus
	FaceTrackingNoFaceFound: EnumTrackerFaceTrackingStatus
	FaceTrackingUnknown: EnumTrackerFaceTrackingStatus
	FaceTrackingLost: EnumTrackerFaceTrackingStatus
	FaceTrackingHasTrackingError: EnumTrackerFaceTrackingStatus
	FaceTrackingIsOccluded: EnumTrackerFaceTrackingStatus
	FaceTrackingUninitialized: EnumTrackerFaceTrackingStatus
	function GetEnumItems(self): { EnumTrackerFaceTrackingStatus }
	function FromName(self, name: string): EnumTrackerFaceTrackingStatus?
	function FromValue(self, value: number): EnumTrackerFaceTrackingStatus?
end

declare extern type EnumTrackerLodFlagMode extends EnumItem with
end

declare extern type EnumTrackerLodFlagMode_INTERNAL extends Enum with
	ForceFalse: EnumTrackerLodFlagMode
	ForceTrue: EnumTrackerLodFlagMode
	Auto: EnumTrackerLodFlagMode
	function GetEnumItems(self): { EnumTrackerLodFlagMode }
	function FromName(self, name: string): EnumTrackerLodFlagMode?
	function FromValue(self, value: number): EnumTrackerLodFlagMode?
end

declare extern type EnumTrackerLodValueMode extends EnumItem with
end

declare extern type EnumTrackerLodValueMode_INTERNAL extends Enum with
	Force0: EnumTrackerLodValueMode
	Force1: EnumTrackerLodValueMode
	Auto: EnumTrackerLodValueMode
	function GetEnumItems(self): { EnumTrackerLodValueMode }
	function FromName(self, name: string): EnumTrackerLodValueMode?
	function FromValue(self, value: number): EnumTrackerLodValueMode?
end

declare extern type EnumTrackerMode extends EnumItem with
end

declare extern type EnumTrackerMode_INTERNAL extends Enum with
	None: EnumTrackerMode
	Audio: EnumTrackerMode
	Video: EnumTrackerMode
	AudioVideo: EnumTrackerMode
	function GetEnumItems(self): { EnumTrackerMode }
	function FromName(self, name: string): EnumTrackerMode?
	function FromValue(self, value: number): EnumTrackerMode?
end

declare extern type EnumTrackerPromptEvent extends EnumItem with
end

declare extern type EnumTrackerPromptEvent_INTERNAL extends Enum with
	LODCameraRecommendDisable: EnumTrackerPromptEvent
	function GetEnumItems(self): { EnumTrackerPromptEvent }
	function FromName(self, name: string): EnumTrackerPromptEvent?
	function FromValue(self, value: number): EnumTrackerPromptEvent?
end

declare extern type EnumTrackerType extends EnumItem with
end

declare extern type EnumTrackerType_INTERNAL extends Enum with
	None: EnumTrackerType
	Face: EnumTrackerType
	UpperBody: EnumTrackerType
	function GetEnumItems(self): { EnumTrackerType }
	function FromName(self, name: string): EnumTrackerType?
	function FromValue(self, value: number): EnumTrackerType?
end

declare extern type EnumTriStateBoolean extends EnumItem with
end

declare extern type EnumTriStateBoolean_INTERNAL extends Enum with
	Unknown: EnumTriStateBoolean
	True: EnumTriStateBoolean
	False: EnumTriStateBoolean
	function GetEnumItems(self): { EnumTriStateBoolean }
	function FromName(self, name: string): EnumTriStateBoolean?
	function FromValue(self, value: number): EnumTriStateBoolean?
end

declare extern type EnumTweenStatus extends EnumItem with
end

declare extern type EnumTweenStatus_INTERNAL extends Enum with
	Canceled: EnumTweenStatus
	Completed: EnumTweenStatus
	function GetEnumItems(self): { EnumTweenStatus }
	function FromName(self, name: string): EnumTweenStatus?
	function FromValue(self, value: number): EnumTweenStatus?
end

declare extern type EnumUICaptureMode extends EnumItem with
end

declare extern type EnumUICaptureMode_INTERNAL extends Enum with
	All: EnumUICaptureMode
	None: EnumUICaptureMode
	function GetEnumItems(self): { EnumUICaptureMode }
	function FromName(self, name: string): EnumUICaptureMode?
	function FromValue(self, value: number): EnumUICaptureMode?
end

declare extern type EnumUIDragDetectorBoundingBehavior extends EnumItem with
end

declare extern type EnumUIDragDetectorBoundingBehavior_INTERNAL extends Enum with
	Automatic: EnumUIDragDetectorBoundingBehavior
	EntireObject: EnumUIDragDetectorBoundingBehavior
	HitPoint: EnumUIDragDetectorBoundingBehavior
	function GetEnumItems(self): { EnumUIDragDetectorBoundingBehavior }
	function FromName(self, name: string): EnumUIDragDetectorBoundingBehavior?
	function FromValue(self, value: number): EnumUIDragDetectorBoundingBehavior?
end

declare extern type EnumUIDragDetectorDragRelativity extends EnumItem with
end

declare extern type EnumUIDragDetectorDragRelativity_INTERNAL extends Enum with
	Absolute: EnumUIDragDetectorDragRelativity
	Relative: EnumUIDragDetectorDragRelativity
	function GetEnumItems(self): { EnumUIDragDetectorDragRelativity }
	function FromName(self, name: string): EnumUIDragDetectorDragRelativity?
	function FromValue(self, value: number): EnumUIDragDetectorDragRelativity?
end

declare extern type EnumUIDragDetectorDragSpace extends EnumItem with
end

declare extern type EnumUIDragDetectorDragSpace_INTERNAL extends Enum with
	Parent: EnumUIDragDetectorDragSpace
	LayerCollector: EnumUIDragDetectorDragSpace
	Reference: EnumUIDragDetectorDragSpace
	function GetEnumItems(self): { EnumUIDragDetectorDragSpace }
	function FromName(self, name: string): EnumUIDragDetectorDragSpace?
	function FromValue(self, value: number): EnumUIDragDetectorDragSpace?
end

declare extern type EnumUIDragDetectorDragStyle extends EnumItem with
end

declare extern type EnumUIDragDetectorDragStyle_INTERNAL extends Enum with
	TranslatePlane: EnumUIDragDetectorDragStyle
	TranslateLine: EnumUIDragDetectorDragStyle
	Rotate: EnumUIDragDetectorDragStyle
	Scriptable: EnumUIDragDetectorDragStyle
	function GetEnumItems(self): { EnumUIDragDetectorDragStyle }
	function FromName(self, name: string): EnumUIDragDetectorDragStyle?
	function FromValue(self, value: number): EnumUIDragDetectorDragStyle?
end

declare extern type EnumUIDragDetectorResponseStyle extends EnumItem with
end

declare extern type EnumUIDragDetectorResponseStyle_INTERNAL extends Enum with
	Offset: EnumUIDragDetectorResponseStyle
	Scale: EnumUIDragDetectorResponseStyle
	CustomOffset: EnumUIDragDetectorResponseStyle
	CustomScale: EnumUIDragDetectorResponseStyle
	function GetEnumItems(self): { EnumUIDragDetectorResponseStyle }
	function FromName(self, name: string): EnumUIDragDetectorResponseStyle?
	function FromValue(self, value: number): EnumUIDragDetectorResponseStyle?
end

declare extern type EnumUIDragSpeedAxisMapping extends EnumItem with
end

declare extern type EnumUIDragSpeedAxisMapping_INTERNAL extends Enum with
	XY: EnumUIDragSpeedAxisMapping
	XX: EnumUIDragSpeedAxisMapping
	YY: EnumUIDragSpeedAxisMapping
	function GetEnumItems(self): { EnumUIDragSpeedAxisMapping }
	function FromName(self, name: string): EnumUIDragSpeedAxisMapping?
	function FromValue(self, value: number): EnumUIDragSpeedAxisMapping?
end

declare extern type EnumUIFlexAlignment extends EnumItem with
end

declare extern type EnumUIFlexAlignment_INTERNAL extends Enum with
	None: EnumUIFlexAlignment
	Fill: EnumUIFlexAlignment
	SpaceAround: EnumUIFlexAlignment
	SpaceBetween: EnumUIFlexAlignment
	SpaceEvenly: EnumUIFlexAlignment
	function GetEnumItems(self): { EnumUIFlexAlignment }
	function FromName(self, name: string): EnumUIFlexAlignment?
	function FromValue(self, value: number): EnumUIFlexAlignment?
end

declare extern type EnumUIFlexMode extends EnumItem with
end

declare extern type EnumUIFlexMode_INTERNAL extends Enum with
	None: EnumUIFlexMode
	Grow: EnumUIFlexMode
	Shrink: EnumUIFlexMode
	Fill: EnumUIFlexMode
	Custom: EnumUIFlexMode
	function GetEnumItems(self): { EnumUIFlexMode }
	function FromName(self, name: string): EnumUIFlexMode?
	function FromValue(self, value: number): EnumUIFlexMode?
end

declare extern type EnumUITheme extends EnumItem with
end

declare extern type EnumUITheme_INTERNAL extends Enum with
	Light: EnumUITheme
	Dark: EnumUITheme
	function GetEnumItems(self): { EnumUITheme }
	function FromName(self, name: string): EnumUITheme?
	function FromValue(self, value: number): EnumUITheme?
end

declare extern type EnumUiMessageType extends EnumItem with
end

declare extern type EnumUiMessageType_INTERNAL extends Enum with
	UiMessageError: EnumUiMessageType
	UiMessageInfo: EnumUiMessageType
	function GetEnumItems(self): { EnumUiMessageType }
	function FromName(self, name: string): EnumUiMessageType?
	function FromValue(self, value: number): EnumUiMessageType?
end

declare extern type EnumUpdateState extends EnumItem with
end

declare extern type EnumUpdateState_INTERNAL extends Enum with
	UpdateNotAvailable: EnumUpdateState
	UpdateAvailable: EnumUpdateState
	UpdateInProgress: EnumUpdateState
	UpdateReady: EnumUpdateState
	UpdateFailed: EnumUpdateState
	function GetEnumItems(self): { EnumUpdateState }
	function FromName(self, name: string): EnumUpdateState?
	function FromValue(self, value: number): EnumUpdateState?
end

declare extern type EnumUploadCaptureResult extends EnumItem with
end

declare extern type EnumUploadCaptureResult_INTERNAL extends Enum with
	Success: EnumUploadCaptureResult
	NeedPermission: EnumUploadCaptureResult
	CaptureModerated: EnumUploadCaptureResult
	CaptureNotInGallery: EnumUploadCaptureResult
	IneligibleCapture: EnumUploadCaptureResult
	UploadQuotaReached: EnumUploadCaptureResult
	UploadPending: EnumUploadCaptureResult
	UploadFailed: EnumUploadCaptureResult
	function GetEnumItems(self): { EnumUploadCaptureResult }
	function FromName(self, name: string): EnumUploadCaptureResult?
	function FromValue(self, value: number): EnumUploadCaptureResult?
end

declare extern type EnumUsageContext extends EnumItem with
end

declare extern type EnumUsageContext_INTERNAL extends Enum with
	Default: EnumUsageContext
	Preview: EnumUsageContext
	function GetEnumItems(self): { EnumUsageContext }
	function FromName(self, name: string): EnumUsageContext?
	function FromValue(self, value: number): EnumUsageContext?
end

declare extern type EnumUserCFrame extends EnumItem with
end

declare extern type EnumUserCFrame_INTERNAL extends Enum with
	Head: EnumUserCFrame
	LeftHand: EnumUserCFrame
	RightHand: EnumUserCFrame
	Floor: EnumUserCFrame
	function GetEnumItems(self): { EnumUserCFrame }
	function FromName(self, name: string): EnumUserCFrame?
	function FromValue(self, value: number): EnumUserCFrame?
end

declare extern type EnumUserInputState extends EnumItem with
end

declare extern type EnumUserInputState_INTERNAL extends Enum with
	Begin: EnumUserInputState
	Change: EnumUserInputState
	End: EnumUserInputState
	Cancel: EnumUserInputState
	None: EnumUserInputState
	function GetEnumItems(self): { EnumUserInputState }
	function FromName(self, name: string): EnumUserInputState?
	function FromValue(self, value: number): EnumUserInputState?
end

declare extern type EnumUserInputType extends EnumItem with
end

declare extern type EnumUserInputType_INTERNAL extends Enum with
	MouseButton1: EnumUserInputType
	MouseButton2: EnumUserInputType
	MouseButton3: EnumUserInputType
	MouseWheel: EnumUserInputType
	MouseMovement: EnumUserInputType
	Touch: EnumUserInputType
	Keyboard: EnumUserInputType
	Focus: EnumUserInputType
	Accelerometer: EnumUserInputType
	Gyro: EnumUserInputType
	Gamepad1: EnumUserInputType
	Gamepad2: EnumUserInputType
	Gamepad3: EnumUserInputType
	Gamepad4: EnumUserInputType
	Gamepad5: EnumUserInputType
	Gamepad6: EnumUserInputType
	Gamepad7: EnumUserInputType
	Gamepad8: EnumUserInputType
	TextInput: EnumUserInputType
	InputMethod: EnumUserInputType
	None: EnumUserInputType
	function GetEnumItems(self): { EnumUserInputType }
	function FromName(self, name: string): EnumUserInputType?
	function FromValue(self, value: number): EnumUserInputType?
end

declare extern type EnumVRComfortSetting extends EnumItem with
end

declare extern type EnumVRComfortSetting_INTERNAL extends Enum with
	Comfort: EnumVRComfortSetting
	Normal: EnumVRComfortSetting
	Expert: EnumVRComfortSetting
	Custom: EnumVRComfortSetting
	function GetEnumItems(self): { EnumVRComfortSetting }
	function FromName(self, name: string): EnumVRComfortSetting?
	function FromValue(self, value: number): EnumVRComfortSetting?
end

declare extern type EnumVRControllerModelMode extends EnumItem with
end

declare extern type EnumVRControllerModelMode_INTERNAL extends Enum with
	Disabled: EnumVRControllerModelMode
	Transparent: EnumVRControllerModelMode
	function GetEnumItems(self): { EnumVRControllerModelMode }
	function FromName(self, name: string): EnumVRControllerModelMode?
	function FromValue(self, value: number): EnumVRControllerModelMode?
end

declare extern type EnumVRDeviceType extends EnumItem with
end

declare extern type EnumVRDeviceType_INTERNAL extends Enum with
	Unknown: EnumVRDeviceType
	OculusRift: EnumVRDeviceType
	HTCVive: EnumVRDeviceType
	ValveIndex: EnumVRDeviceType
	OculusQuest: EnumVRDeviceType
	function GetEnumItems(self): { EnumVRDeviceType }
	function FromName(self, name: string): EnumVRDeviceType?
	function FromValue(self, value: number): EnumVRDeviceType?
end

declare extern type EnumVRLaserPointerMode extends EnumItem with
end

declare extern type EnumVRLaserPointerMode_INTERNAL extends Enum with
	Disabled: EnumVRLaserPointerMode
	Pointer: EnumVRLaserPointerMode
	DualPointer: EnumVRLaserPointerMode
	function GetEnumItems(self): { EnumVRLaserPointerMode }
	function FromName(self, name: string): EnumVRLaserPointerMode?
	function FromValue(self, value: number): EnumVRLaserPointerMode?
end

declare extern type EnumVRSafetyBubbleMode extends EnumItem with
end

declare extern type EnumVRSafetyBubbleMode_INTERNAL extends Enum with
	NoOne: EnumVRSafetyBubbleMode
	OnlyFriends: EnumVRSafetyBubbleMode
	Anyone: EnumVRSafetyBubbleMode
	function GetEnumItems(self): { EnumVRSafetyBubbleMode }
	function FromName(self, name: string): EnumVRSafetyBubbleMode?
	function FromValue(self, value: number): EnumVRSafetyBubbleMode?
end

declare extern type EnumVRScaling extends EnumItem with
end

declare extern type EnumVRScaling_INTERNAL extends Enum with
	World: EnumVRScaling
	Off: EnumVRScaling
	function GetEnumItems(self): { EnumVRScaling }
	function FromName(self, name: string): EnumVRScaling?
	function FromValue(self, value: number): EnumVRScaling?
end

declare extern type EnumVRSessionState extends EnumItem with
end

declare extern type EnumVRSessionState_INTERNAL extends Enum with
	Undefined: EnumVRSessionState
	Idle: EnumVRSessionState
	Visible: EnumVRSessionState
	Focused: EnumVRSessionState
	Stopping: EnumVRSessionState
	function GetEnumItems(self): { EnumVRSessionState }
	function FromName(self, name: string): EnumVRSessionState?
	function FromValue(self, value: number): EnumVRSessionState?
end

declare extern type EnumVRTouchpad extends EnumItem with
end

declare extern type EnumVRTouchpad_INTERNAL extends Enum with
	Left: EnumVRTouchpad
	Right: EnumVRTouchpad
	function GetEnumItems(self): { EnumVRTouchpad }
	function FromName(self, name: string): EnumVRTouchpad?
	function FromValue(self, value: number): EnumVRTouchpad?
end

declare extern type EnumVRTouchpadMode extends EnumItem with
end

declare extern type EnumVRTouchpadMode_INTERNAL extends Enum with
	Touch: EnumVRTouchpadMode
	VirtualThumbstick: EnumVRTouchpadMode
	ABXY: EnumVRTouchpadMode
	function GetEnumItems(self): { EnumVRTouchpadMode }
	function FromName(self, name: string): EnumVRTouchpadMode?
	function FromValue(self, value: number): EnumVRTouchpadMode?
end

declare extern type EnumVelocityConstraintMode extends EnumItem with
end

declare extern type EnumVelocityConstraintMode_INTERNAL extends Enum with
	Line: EnumVelocityConstraintMode
	Plane: EnumVelocityConstraintMode
	Vector: EnumVelocityConstraintMode
	function GetEnumItems(self): { EnumVelocityConstraintMode }
	function FromName(self, name: string): EnumVelocityConstraintMode?
	function FromValue(self, value: number): EnumVelocityConstraintMode?
end

declare extern type EnumVerticalAlignment extends EnumItem with
end

declare extern type EnumVerticalAlignment_INTERNAL extends Enum with
	Center: EnumVerticalAlignment
	Top: EnumVerticalAlignment
	Bottom: EnumVerticalAlignment
	function GetEnumItems(self): { EnumVerticalAlignment }
	function FromName(self, name: string): EnumVerticalAlignment?
	function FromValue(self, value: number): EnumVerticalAlignment?
end

declare extern type EnumVerticalScrollBarPosition extends EnumItem with
end

declare extern type EnumVerticalScrollBarPosition_INTERNAL extends Enum with
	Right: EnumVerticalScrollBarPosition
	Left: EnumVerticalScrollBarPosition
	function GetEnumItems(self): { EnumVerticalScrollBarPosition }
	function FromName(self, name: string): EnumVerticalScrollBarPosition?
	function FromValue(self, value: number): EnumVerticalScrollBarPosition?
end

declare extern type EnumVibrationMotor extends EnumItem with
end

declare extern type EnumVibrationMotor_INTERNAL extends Enum with
	Large: EnumVibrationMotor
	Small: EnumVibrationMotor
	LeftTrigger: EnumVibrationMotor
	RightTrigger: EnumVibrationMotor
	LeftHand: EnumVibrationMotor
	RightHand: EnumVibrationMotor
	function GetEnumItems(self): { EnumVibrationMotor }
	function FromName(self, name: string): EnumVibrationMotor?
	function FromValue(self, value: number): EnumVibrationMotor?
end

declare extern type EnumVideoCaptureResult extends EnumItem with
end

declare extern type EnumVideoCaptureResult_INTERNAL extends Enum with
	Success: EnumVideoCaptureResult
	OtherError: EnumVideoCaptureResult
	ScreenSizeChanged: EnumVideoCaptureResult
	TimeLimitReached: EnumVideoCaptureResult
	function GetEnumItems(self): { EnumVideoCaptureResult }
	function FromName(self, name: string): EnumVideoCaptureResult?
	function FromValue(self, value: number): EnumVideoCaptureResult?
end

declare extern type EnumVideoCaptureStartedResult extends EnumItem with
end

declare extern type EnumVideoCaptureStartedResult_INTERNAL extends Enum with
	Success: EnumVideoCaptureStartedResult
	OtherError: EnumVideoCaptureStartedResult
	CapturingAlready: EnumVideoCaptureStartedResult
	NoDeviceSupport: EnumVideoCaptureStartedResult
	NoSpaceOnDevice: EnumVideoCaptureStartedResult
	function GetEnumItems(self): { EnumVideoCaptureStartedResult }
	function FromName(self, name: string): EnumVideoCaptureStartedResult?
	function FromValue(self, value: number): EnumVideoCaptureStartedResult?
end

declare extern type EnumVideoDeviceCaptureQuality extends EnumItem with
end

declare extern type EnumVideoDeviceCaptureQuality_INTERNAL extends Enum with
	Default: EnumVideoDeviceCaptureQuality
	Low: EnumVideoDeviceCaptureQuality
	Medium: EnumVideoDeviceCaptureQuality
	High: EnumVideoDeviceCaptureQuality
	function GetEnumItems(self): { EnumVideoDeviceCaptureQuality }
	function FromName(self, name: string): EnumVideoDeviceCaptureQuality?
	function FromValue(self, value: number): EnumVideoDeviceCaptureQuality?
end

declare extern type EnumVideoError extends EnumItem with
end

declare extern type EnumVideoError_INTERNAL extends Enum with
	Ok: EnumVideoError
	Eof: EnumVideoError
	EAgain: EnumVideoError
	BadParameter: EnumVideoError
	AllocFailed: EnumVideoError
	CodecInitFailed: EnumVideoError
	CodecCloseFailed: EnumVideoError
	DecodeFailed: EnumVideoError
	ParsingFailed: EnumVideoError
	Unsupported: EnumVideoError
	Generic: EnumVideoError
	DownloadFailed: EnumVideoError
	StreamNotFound: EnumVideoError
	EncodeFailed: EnumVideoError
	CreateFailed: EnumVideoError
	NoPermission: EnumVideoError
	NoService: EnumVideoError
	ReleaseFailed: EnumVideoError
	Unknown: EnumVideoError
	function GetEnumItems(self): { EnumVideoError }
	function FromName(self, name: string): EnumVideoError?
	function FromValue(self, value: number): EnumVideoError?
end

declare extern type EnumVideoSampleSize extends EnumItem with
end

declare extern type EnumVideoSampleSize_INTERNAL extends Enum with
	Small: EnumVideoSampleSize
	Medium: EnumVideoSampleSize
	Large: EnumVideoSampleSize
	Full: EnumVideoSampleSize
	function GetEnumItems(self): { EnumVideoSampleSize }
	function FromName(self, name: string): EnumVideoSampleSize?
	function FromValue(self, value: number): EnumVideoSampleSize?
end

declare extern type EnumViewMode extends EnumItem with
end

declare extern type EnumViewMode_INTERNAL extends Enum with
	None: EnumViewMode
	GeometryComplexity: EnumViewMode
	Transparent: EnumViewMode
	Decal: EnumViewMode
	function GetEnumItems(self): { EnumViewMode }
	function FromName(self, name: string): EnumViewMode?
	function FromValue(self, value: number): EnumViewMode?
end

declare extern type EnumVirtualCursorMode extends EnumItem with
end

declare extern type EnumVirtualCursorMode_INTERNAL extends Enum with
	Default: EnumVirtualCursorMode
	Disabled: EnumVirtualCursorMode
	Enabled: EnumVirtualCursorMode
	function GetEnumItems(self): { EnumVirtualCursorMode }
	function FromName(self, name: string): EnumVirtualCursorMode?
	function FromValue(self, value: number): EnumVirtualCursorMode?
end

declare extern type EnumVirtualInputMode extends EnumItem with
end

declare extern type EnumVirtualInputMode_INTERNAL extends Enum with
	None: EnumVirtualInputMode
	Recording: EnumVirtualInputMode
	Playing: EnumVirtualInputMode
	function GetEnumItems(self): { EnumVirtualInputMode }
	function FromName(self, name: string): EnumVirtualInputMode?
	function FromValue(self, value: number): EnumVirtualInputMode?
end

declare extern type EnumVoiceChatDistanceAttenuationType extends EnumItem with
end

declare extern type EnumVoiceChatDistanceAttenuationType_INTERNAL extends Enum with
	Inverse: EnumVoiceChatDistanceAttenuationType
	Legacy: EnumVoiceChatDistanceAttenuationType
	function GetEnumItems(self): { EnumVoiceChatDistanceAttenuationType }
	function FromName(self, name: string): EnumVoiceChatDistanceAttenuationType?
	function FromValue(self, value: number): EnumVoiceChatDistanceAttenuationType?
end

declare extern type EnumVoiceChatState extends EnumItem with
end

declare extern type EnumVoiceChatState_INTERNAL extends Enum with
	Idle: EnumVoiceChatState
	Joining: EnumVoiceChatState
	JoiningRetry: EnumVoiceChatState
	Joined: EnumVoiceChatState
	Leaving: EnumVoiceChatState
	Ended: EnumVoiceChatState
	Failed: EnumVoiceChatState
	function GetEnumItems(self): { EnumVoiceChatState }
	function FromName(self, name: string): EnumVoiceChatState?
	function FromValue(self, value: number): EnumVoiceChatState?
end

declare extern type EnumVoiceClientLeaveReasons extends EnumItem with
end

declare extern type EnumVoiceClientLeaveReasons_INTERNAL extends Enum with
	Unknown: EnumVoiceClientLeaveReasons
	ClientNetworkDisconnected: EnumVoiceClientLeaveReasons
	PlayerLeft: EnumVoiceClientLeaveReasons
	ClientShutdown: EnumVoiceClientLeaveReasons
	PublishFailed: EnumVoiceClientLeaveReasons
	RejoinReceived: EnumVoiceClientLeaveReasons
	VoiceReboot: EnumVoiceClientLeaveReasons
	ImguiDebugLeave: EnumVoiceClientLeaveReasons
	LuaInitiated: EnumVoiceClientLeaveReasons
	function GetEnumItems(self): { EnumVoiceClientLeaveReasons }
	function FromName(self, name: string): EnumVoiceClientLeaveReasons?
	function FromValue(self, value: number): EnumVoiceClientLeaveReasons?
end

declare extern type EnumVoiceControlPath extends EnumItem with
end

declare extern type EnumVoiceControlPath_INTERNAL extends Enum with
	Publish: EnumVoiceControlPath
	Subscribe: EnumVoiceControlPath
	Join: EnumVoiceControlPath
	function GetEnumItems(self): { EnumVoiceControlPath }
	function FromName(self, name: string): EnumVoiceControlPath?
	function FromValue(self, value: number): EnumVoiceControlPath?
end

declare extern type EnumVoiceRccReconnectReason extends EnumItem with
end

declare extern type EnumVoiceRccReconnectReason_INTERNAL extends Enum with
	Unknown: EnumVoiceRccReconnectReason
	Migration: EnumVoiceRccReconnectReason
	CloseRoom: EnumVoiceRccReconnectReason
	FAEUpdate: EnumVoiceRccReconnectReason
	BlockListChanged: EnumVoiceRccReconnectReason
	function GetEnumItems(self): { EnumVoiceRccReconnectReason }
	function FromName(self, name: string): EnumVoiceRccReconnectReason?
	function FromValue(self, value: number): EnumVoiceRccReconnectReason?
end

declare extern type EnumVolumetricAudio extends EnumItem with
end

declare extern type EnumVolumetricAudio_INTERNAL extends Enum with
	Disabled: EnumVolumetricAudio
	Automatic: EnumVolumetricAudio
	Enabled: EnumVolumetricAudio
	function GetEnumItems(self): { EnumVolumetricAudio }
	function FromName(self, name: string): EnumVolumetricAudio?
	function FromValue(self, value: number): EnumVolumetricAudio?
end

declare extern type EnumWaterDirection extends EnumItem with
end

declare extern type EnumWaterDirection_INTERNAL extends Enum with
	NegX: EnumWaterDirection
	X: EnumWaterDirection
	NegY: EnumWaterDirection
	Y: EnumWaterDirection
	NegZ: EnumWaterDirection
	Z: EnumWaterDirection
	function GetEnumItems(self): { EnumWaterDirection }
	function FromName(self, name: string): EnumWaterDirection?
	function FromValue(self, value: number): EnumWaterDirection?
end

declare extern type EnumWaterForce extends EnumItem with
end

declare extern type EnumWaterForce_INTERNAL extends Enum with
	None: EnumWaterForce
	Small: EnumWaterForce
	Medium: EnumWaterForce
	Strong: EnumWaterForce
	Max: EnumWaterForce
	function GetEnumItems(self): { EnumWaterForce }
	function FromName(self, name: string): EnumWaterForce?
	function FromValue(self, value: number): EnumWaterForce?
end

declare extern type EnumWebSocketState extends EnumItem with
end

declare extern type EnumWebSocketState_INTERNAL extends Enum with
	Connecting: EnumWebSocketState
	Open: EnumWebSocketState
	Closing: EnumWebSocketState
	Closed: EnumWebSocketState
	function GetEnumItems(self): { EnumWebSocketState }
	function FromName(self, name: string): EnumWebSocketState?
	function FromValue(self, value: number): EnumWebSocketState?
end

declare extern type EnumWebStreamClientState extends EnumItem with
end

declare extern type EnumWebStreamClientState_INTERNAL extends Enum with
	Connecting: EnumWebStreamClientState
	Open: EnumWebStreamClientState
	Error: EnumWebStreamClientState
	Closed: EnumWebStreamClientState
	function GetEnumItems(self): { EnumWebStreamClientState }
	function FromName(self, name: string): EnumWebStreamClientState?
	function FromValue(self, value: number): EnumWebStreamClientState?
end

declare extern type EnumWebStreamClientType extends EnumItem with
end

declare extern type EnumWebStreamClientType_INTERNAL extends Enum with
	SSE: EnumWebStreamClientType
	RawStream: EnumWebStreamClientType
	WebSocket: EnumWebStreamClientType
	function GetEnumItems(self): { EnumWebStreamClientType }
	function FromName(self, name: string): EnumWebStreamClientType?
	function FromValue(self, value: number): EnumWebStreamClientType?
end

declare extern type EnumWeldConstraintPreserve extends EnumItem with
end

declare extern type EnumWeldConstraintPreserve_INTERNAL extends Enum with
	All: EnumWeldConstraintPreserve
	None: EnumWeldConstraintPreserve
	Touching: EnumWeldConstraintPreserve
	function GetEnumItems(self): { EnumWeldConstraintPreserve }
	function FromName(self, name: string): EnumWeldConstraintPreserve?
	function FromValue(self, value: number): EnumWeldConstraintPreserve?
end

declare extern type EnumWhenUserFirstPlayed extends EnumItem with
end

declare extern type EnumWhenUserFirstPlayed_INTERNAL extends Enum with
	Unknown: EnumWhenUserFirstPlayed
	Days0To30: EnumWhenUserFirstPlayed
	Days31To90: EnumWhenUserFirstPlayed
	Days91To180: EnumWhenUserFirstPlayed
	Days181To365: EnumWhenUserFirstPlayed
	Days366Plus: EnumWhenUserFirstPlayed
	function GetEnumItems(self): { EnumWhenUserFirstPlayed }
	function FromName(self, name: string): EnumWhenUserFirstPlayed?
	function FromValue(self, value: number): EnumWhenUserFirstPlayed?
end

declare extern type EnumWhisperChatPrivacyMode extends EnumItem with
end

declare extern type EnumWhisperChatPrivacyMode_INTERNAL extends Enum with
	AllUsers: EnumWhisperChatPrivacyMode
	NoOne: EnumWhisperChatPrivacyMode
	function GetEnumItems(self): { EnumWhisperChatPrivacyMode }
	function FromName(self, name: string): EnumWhisperChatPrivacyMode?
	function FromValue(self, value: number): EnumWhisperChatPrivacyMode?
end

declare extern type EnumWrapLayerAutoSkin extends EnumItem with
end

declare extern type EnumWrapLayerAutoSkin_INTERNAL extends Enum with
	Disabled: EnumWrapLayerAutoSkin
	EnabledPreserve: EnumWrapLayerAutoSkin
	EnabledOverride: EnumWrapLayerAutoSkin
	function GetEnumItems(self): { EnumWrapLayerAutoSkin }
	function FromName(self, name: string): EnumWrapLayerAutoSkin?
	function FromValue(self, value: number): EnumWrapLayerAutoSkin?
end

declare extern type EnumWrapLayerDebugMode extends EnumItem with
end

declare extern type EnumWrapLayerDebugMode_INTERNAL extends Enum with
	None: EnumWrapLayerDebugMode
	BoundCage: EnumWrapLayerDebugMode
	LayerCage: EnumWrapLayerDebugMode
	BoundCageAndLinks: EnumWrapLayerDebugMode
	Reference: EnumWrapLayerDebugMode
	Rbf: EnumWrapLayerDebugMode
	OuterCage: EnumWrapLayerDebugMode
	ReferenceMeshAfterMorph: EnumWrapLayerDebugMode
	HSROuterDetail: EnumWrapLayerDebugMode
	HSROuter: EnumWrapLayerDebugMode
	HSRInner: EnumWrapLayerDebugMode
	HSRInnerReverse: EnumWrapLayerDebugMode
	LayerCageFittedToBase: EnumWrapLayerDebugMode
	LayerCageFittedToPrev: EnumWrapLayerDebugMode
	PreWrapDeformerOuterCage: EnumWrapLayerDebugMode
	SkinningTransfer: EnumWrapLayerDebugMode
	function GetEnumItems(self): { EnumWrapLayerDebugMode }
	function FromName(self, name: string): EnumWrapLayerDebugMode?
	function FromValue(self, value: number): EnumWrapLayerDebugMode?
end

declare extern type EnumWrapTargetDebugMode extends EnumItem with
end

declare extern type EnumWrapTargetDebugMode_INTERNAL extends Enum with
	None: EnumWrapTargetDebugMode
	TargetCageOriginal: EnumWrapTargetDebugMode
	TargetCageCompressed: EnumWrapTargetDebugMode
	TargetCageInterface: EnumWrapTargetDebugMode
	TargetLayerCageOriginal: EnumWrapTargetDebugMode
	TargetLayerCageCompressed: EnumWrapTargetDebugMode
	TargetLayerInterface: EnumWrapTargetDebugMode
	Rbf: EnumWrapTargetDebugMode
	OuterCageDetail: EnumWrapTargetDebugMode
	PreWrapDeformerCage: EnumWrapTargetDebugMode
	function GetEnumItems(self): { EnumWrapTargetDebugMode }
	function FromName(self, name: string): EnumWrapTargetDebugMode?
	function FromValue(self, value: number): EnumWrapTargetDebugMode?
end

declare extern type EnumZIndexBehavior extends EnumItem with
end

declare extern type EnumZIndexBehavior_INTERNAL extends Enum with
	Global: EnumZIndexBehavior
	Sibling: EnumZIndexBehavior
	function GetEnumItems(self): { EnumZIndexBehavior }
	function FromName(self, name: string): EnumZIndexBehavior?
	function FromValue(self, value: number): EnumZIndexBehavior?
end

declare Enum: {
	AccessModifierType: EnumAccessModifierType_INTERNAL,
	AccessoryType: EnumAccessoryType_INTERNAL,
	ActionOnAutoResumeSync: EnumActionOnAutoResumeSync_INTERNAL,
	ActionOnStopSync: EnumActionOnStopSync_INTERNAL,
	ActionType: EnumActionType_INTERNAL,
	ActivePayerStatus: EnumActivePayerStatus_INTERNAL,
	ActuatorRelativeTo: EnumActuatorRelativeTo_INTERNAL,
	ActuatorType: EnumActuatorType_INTERNAL,
	AdAvailabilityResult: EnumAdAvailabilityResult_INTERNAL,
	AdEventType: EnumAdEventType_INTERNAL,
	AdFormat: EnumAdFormat_INTERNAL,
	AdShape: EnumAdShape_INTERNAL,
	AdTeleportMethod: EnumAdTeleportMethod_INTERNAL,
	AdUIEventType: EnumAdUIEventType_INTERNAL,
	AdUIType: EnumAdUIType_INTERNAL,
	AdUnitStatus: EnumAdUnitStatus_INTERNAL,
	AdornCullingMode: EnumAdornCullingMode_INTERNAL,
	AdornShading: EnumAdornShading_INTERNAL,
	AgeCheckStatus: EnumAgeCheckStatus_INTERNAL,
	AlignType: EnumAlignType_INTERNAL,
	AlphaMode: EnumAlphaMode_INTERNAL,
	AnalyticsCustomFieldKeys: EnumAnalyticsCustomFieldKeys_INTERNAL,
	AnalyticsEconomyAction: EnumAnalyticsEconomyAction_INTERNAL,
	AnalyticsEconomyFlowType: EnumAnalyticsEconomyFlowType_INTERNAL,
	AnalyticsEconomyTransactionType: EnumAnalyticsEconomyTransactionType_INTERNAL,
	AnalyticsLogLevel: EnumAnalyticsLogLevel_INTERNAL,
	AnalyticsProgressionStatus: EnumAnalyticsProgressionStatus_INTERNAL,
	AnalyticsProgressionType: EnumAnalyticsProgressionType_INTERNAL,
	AnimationClipFromVideoStatus: EnumAnimationClipFromVideoStatus_INTERNAL,
	AnimationNodeBlend2DInputMode: EnumAnimationNodeBlend2DInputMode_INTERNAL,
	AnimationNodeInterruptible: EnumAnimationNodeInterruptible_INTERNAL,
	AnimationNodePhaseSync: EnumAnimationNodePhaseSync_INTERNAL,
	AnimationNodePlayMode: EnumAnimationNodePlayMode_INTERNAL,
	AnimationNodeTransitionType: EnumAnimationNodeTransitionType_INTERNAL,
	AnimationNodeType: EnumAnimationNodeType_INTERNAL,
	AnimationNodeWaitFor: EnumAnimationNodeWaitFor_INTERNAL,
	AnimationPriority: EnumAnimationPriority_INTERNAL,
	AnimatorRetargetingMode: EnumAnimatorRetargetingMode_INTERNAL,
	AnnotationChannelContentPreference: EnumAnnotationChannelContentPreference_INTERNAL,
	AnnotationEditingMode: EnumAnnotationEditingMode_INTERNAL,
	AnnotationPlaceContentPreference: EnumAnnotationPlaceContentPreference_INTERNAL,
	AnnotationRequestStatus: EnumAnnotationRequestStatus_INTERNAL,
	AnnotationRequestType: EnumAnnotationRequestType_INTERNAL,
	AntiAliasing: EnumAntiAliasing_INTERNAL,
	AppLifecycleManagerState: EnumAppLifecycleManagerState_INTERNAL,
	AppShellActionType: EnumAppShellActionType_INTERNAL,
	AppShellFeature: EnumAppShellFeature_INTERNAL,
	AppUpdateStatus: EnumAppUpdateStatus_INTERNAL,
	ApplyStrokeMode: EnumApplyStrokeMode_INTERNAL,
	AspectType: EnumAspectType_INTERNAL,
	AssetCreatorType: EnumAssetCreatorType_INTERNAL,
	AssetFetchStatus: EnumAssetFetchStatus_INTERNAL,
	AssetRepresentation: EnumAssetRepresentation_INTERNAL,
	AssetType: EnumAssetType_INTERNAL,
	AssetTypeVerification: EnumAssetTypeVerification_INTERNAL,
	AudioApiRollout: EnumAudioApiRollout_INTERNAL,
	AudioCaptureMode: EnumAudioCaptureMode_INTERNAL,
	AudioChannelLayout: EnumAudioChannelLayout_INTERNAL,
	AudioFilterType: EnumAudioFilterType_INTERNAL,
	AudioSimulationFidelity: EnumAudioSimulationFidelity_INTERNAL,
	AudioSubType: EnumAudioSubType_INTERNAL,
	AudioWindowSize: EnumAudioWindowSize_INTERNAL,
	AuthorityMode: EnumAuthorityMode_INTERNAL,
	AutoIndentRule: EnumAutoIndentRule_INTERNAL,
	AutomaticSize: EnumAutomaticSize_INTERNAL,
	AvatarAssetType: EnumAvatarAssetType_INTERNAL,
	AvatarChatServiceFeature: EnumAvatarChatServiceFeature_INTERNAL,
	AvatarContextMenuOption: EnumAvatarContextMenuOption_INTERNAL,
	AvatarGenerationError: EnumAvatarGenerationError_INTERNAL,
	AvatarItemType: EnumAvatarItemType_INTERNAL,
	AvatarPromptResult: EnumAvatarPromptResult_INTERNAL,
	AvatarSettingsAccessoryLimitMethod: EnumAvatarSettingsAccessoryLimitMethod_INTERNAL,
	AvatarSettingsAccessoryMode: EnumAvatarSettingsAccessoryMode_INTERNAL,
	AvatarSettingsAnimationClipsMode: EnumAvatarSettingsAnimationClipsMode_INTERNAL,
	AvatarSettingsAnimationPacksMode: EnumAvatarSettingsAnimationPacksMode_INTERNAL,
	AvatarSettingsAppearanceMode: EnumAvatarSettingsAppearanceMode_INTERNAL,
	AvatarSettingsBuildMode: EnumAvatarSettingsBuildMode_INTERNAL,
	AvatarSettingsCharacterControllerMode: EnumAvatarSettingsCharacterControllerMode_INTERNAL,
	AvatarSettingsClothingMode: EnumAvatarSettingsClothingMode_INTERNAL,
	AvatarSettingsCollisionMode: EnumAvatarSettingsCollisionMode_INTERNAL,
	AvatarSettingsCustomAccessoryMode: EnumAvatarSettingsCustomAccessoryMode_INTERNAL,
	AvatarSettingsCustomBodyType: EnumAvatarSettingsCustomBodyType_INTERNAL,
	AvatarSettingsCustomClothingMode: EnumAvatarSettingsCustomClothingMode_INTERNAL,
	AvatarSettingsHitAndTouchDetectionMode: EnumAvatarSettingsHitAndTouchDetectionMode_INTERNAL,
	AvatarSettingsJumpMode: EnumAvatarSettingsJumpMode_INTERNAL,
	AvatarSettingsLegacyCollisionMode: EnumAvatarSettingsLegacyCollisionMode_INTERNAL,
	AvatarSettingsScaleMode: EnumAvatarSettingsScaleMode_INTERNAL,
	AvatarThumbnailCustomizationType: EnumAvatarThumbnailCustomizationType_INTERNAL,
	AvatarUnificationMode: EnumAvatarUnificationMode_INTERNAL,
	Axis: EnumAxis_INTERNAL,
	BenefitType: EnumBenefitType_INTERNAL,
	BinType: EnumBinType_INTERNAL,
	BlendMode: EnumBlendMode_INTERNAL,
	BodyPart: EnumBodyPart_INTERNAL,
	BodyPartR15: EnumBodyPartR15_INTERNAL,
	BorderMode: EnumBorderMode_INTERNAL,
	BorderStrokePosition: EnumBorderStrokePosition_INTERNAL,
	BreakReason: EnumBreakReason_INTERNAL,
	BreakpointRemoveReason: EnumBreakpointRemoveReason_INTERNAL,
	BulkMoveMode: EnumBulkMoveMode_INTERNAL,
	BundleType: EnumBundleType_INTERNAL,
	Button: EnumButton_INTERNAL,
	ButtonStyle: EnumButtonStyle_INTERNAL,
	CageType: EnumCageType_INTERNAL,
	CameraMode: EnumCameraMode_INTERNAL,
	CameraNavigationModel: EnumCameraNavigationModel_INTERNAL,
	CameraPanMode: EnumCameraPanMode_INTERNAL,
	CameraSpeedAdjustBinding: EnumCameraSpeedAdjustBinding_INTERNAL,
	CameraType: EnumCameraType_INTERNAL,
	CanCollaborateError: EnumCanCollaborateError_INTERNAL,
	CaptureGalleryPermission: EnumCaptureGalleryPermission_INTERNAL,
	CaptureType: EnumCaptureType_INTERNAL,
	CatalogCategoryFilter: EnumCatalogCategoryFilter_INTERNAL,
	CatalogSortAggregation: EnumCatalogSortAggregation_INTERNAL,
	CatalogSortType: EnumCatalogSortType_INTERNAL,
	CellBlock: EnumCellBlock_INTERNAL,
	CellMaterial: EnumCellMaterial_INTERNAL,
	CellOrientation: EnumCellOrientation_INTERNAL,
	CenterDialogType: EnumCenterDialogType_INTERNAL,
	CharacterControlMode: EnumCharacterControlMode_INTERNAL,
	ChatCallbackType: EnumChatCallbackType_INTERNAL,
	ChatColor: EnumChatColor_INTERNAL,
	ChatMode: EnumChatMode_INTERNAL,
	ChatPrivacyMode: EnumChatPrivacyMode_INTERNAL,
	ChatRestrictionStatus: EnumChatRestrictionStatus_INTERNAL,
	ChatStyle: EnumChatStyle_INTERNAL,
	ChatVersion: EnumChatVersion_INTERNAL,
	ClientAnimatorThrottlingMode: EnumClientAnimatorThrottlingMode_INTERNAL,
	CloseReason: EnumCloseReason_INTERNAL,
	CollaboratorStatus: EnumCollaboratorStatus_INTERNAL,
	CollisionFidelity: EnumCollisionFidelity_INTERNAL,
	CommandPermission: EnumCommandPermission_INTERNAL,
	CompileTarget: EnumCompileTarget_INTERNAL,
	CompletionAcceptanceBehavior: EnumCompletionAcceptanceBehavior_INTERNAL,
	CompletionItemKind: EnumCompletionItemKind_INTERNAL,
	CompletionItemTag: EnumCompletionItemTag_INTERNAL,
	CompletionTriggerKind: EnumCompletionTriggerKind_INTERNAL,
	CompositeValueCurveType: EnumCompositeValueCurveType_INTERNAL,
	CompressionAlgorithm: EnumCompressionAlgorithm_INTERNAL,
	ComputerCameraMovementMode: EnumComputerCameraMovementMode_INTERNAL,
	ComputerMovementMode: EnumComputerMovementMode_INTERNAL,
	ConfigSnapshotErrorState: EnumConfigSnapshotErrorState_INTERNAL,
	ConnectionError: EnumConnectionError_INTERNAL,
	ConnectionState: EnumConnectionState_INTERNAL,
	ContentSourceType: EnumContentSourceType_INTERNAL,
	ContextActionPriority: EnumContextActionPriority_INTERNAL,
	ContextActionResult: EnumContextActionResult_INTERNAL,
	ControlMode: EnumControlMode_INTERNAL,
	CoreGuiType: EnumCoreGuiType_INTERNAL,
	CreateAssetResult: EnumCreateAssetResult_INTERNAL,
	CreateContentResult: EnumCreateContentResult_INTERNAL,
	CreateOutfitFailure: EnumCreateOutfitFailure_INTERNAL,
	CreatorType: EnumCreatorType_INTERNAL,
	CreatorTypeFilter: EnumCreatorTypeFilter_INTERNAL,
	CurrencyType: EnumCurrencyType_INTERNAL,
	CustomCameraMode: EnumCustomCameraMode_INTERNAL,
	DataModelExtractorFileType: EnumDataModelExtractorFileType_INTERNAL,
	DataStoreRequestType: EnumDataStoreRequestType_INTERNAL,
	DebugBreakModeType: EnumDebugBreakModeType_INTERNAL,
	DebuggerEndReason: EnumDebuggerEndReason_INTERNAL,
	DebuggerExceptionBreakMode: EnumDebuggerExceptionBreakMode_INTERNAL,
	DebuggerFrameType: EnumDebuggerFrameType_INTERNAL,
	DebuggerPauseReason: EnumDebuggerPauseReason_INTERNAL,
	DebuggerResumeType: EnumDebuggerResumeType_INTERNAL,
	DebuggerStatus: EnumDebuggerStatus_INTERNAL,
	DefaultScriptSyncFileType: EnumDefaultScriptSyncFileType_INTERNAL,
	DevCameraOcclusionMode: EnumDevCameraOcclusionMode_INTERNAL,
	DevComputerCameraMovementMode: EnumDevComputerCameraMovementMode_INTERNAL,
	DevComputerMovementMode: EnumDevComputerMovementMode_INTERNAL,
	DevTouchCameraMovementMode: EnumDevTouchCameraMovementMode_INTERNAL,
	DevTouchMovementMode: EnumDevTouchMovementMode_INTERNAL,
	DeveloperMemoryTag: EnumDeveloperMemoryTag_INTERNAL,
	DeviceFeatureType: EnumDeviceFeatureType_INTERNAL,
	DeviceForm: EnumDeviceForm_INTERNAL,
	DeviceLevel: EnumDeviceLevel_INTERNAL,
	DeviceSimulatorScalingMode: EnumDeviceSimulatorScalingMode_INTERNAL,
	DeviceType: EnumDeviceType_INTERNAL,
	DialogBehaviorType: EnumDialogBehaviorType_INTERNAL,
	DialogPurpose: EnumDialogPurpose_INTERNAL,
	DialogTone: EnumDialogTone_INTERNAL,
	DigitsRigDescriptionSide: EnumDigitsRigDescriptionSide_INTERNAL,
	DiscountType: EnumDiscountType_INTERNAL,
	DisplayScalingMode: EnumDisplayScalingMode_INTERNAL,
	DisplaySize: EnumDisplaySize_INTERNAL,
	DomainType: EnumDomainType_INTERNAL,
	DominantAxis: EnumDominantAxis_INTERNAL,
	DraftStatusCode: EnumDraftStatusCode_INTERNAL,
	DragDetectorDragStyle: EnumDragDetectorDragStyle_INTERNAL,
	DragDetectorPermissionPolicy: EnumDragDetectorPermissionPolicy_INTERNAL,
	DragDetectorResponseStyle: EnumDragDetectorResponseStyle_INTERNAL,
	DraggerCoordinateSpace: EnumDraggerCoordinateSpace_INTERNAL,
	DraggerMovementMode: EnumDraggerMovementMode_INTERNAL,
	DraggingScrollBar: EnumDraggingScrollBar_INTERNAL,
	EasingDirection: EnumEasingDirection_INTERNAL,
	EasingStyle: EnumEasingStyle_INTERNAL,
	EditableStatus: EnumEditableStatus_INTERNAL,
	ElasticBehavior: EnumElasticBehavior_INTERNAL,
	EmitterPositionType: EnumEmitterPositionType_INTERNAL,
	EngagementLevel: EnumEngagementLevel_INTERNAL,
	EngineFolder: EnumEngineFolder_INTERNAL,
	EnviromentalPhysicsThrottle: EnumEnviromentalPhysicsThrottle_INTERNAL,
	ExperienceActivationStatus: EnumExperienceActivationStatus_INTERNAL,
	ExperienceAuthScope: EnumExperienceAuthScope_INTERNAL,
	ExperienceEventStatus: EnumExperienceEventStatus_INTERNAL,
	ExperienceStateCaptureSelectionMode: EnumExperienceStateCaptureSelectionMode_INTERNAL,
	ExperienceStateRecordingLoadMode: EnumExperienceStateRecordingLoadMode_INTERNAL,
	ExperienceStateRecordingLoadSourceType: EnumExperienceStateRecordingLoadSourceType_INTERNAL,
	ExperienceStateRecordingPlaybackMode: EnumExperienceStateRecordingPlaybackMode_INTERNAL,
	ExplosionType: EnumExplosionType_INTERNAL,
	ExternalEditorMode: EnumExternalEditorMode_INTERNAL,
	FACSDataLod: EnumFACSDataLod_INTERNAL,
	FacialAgeEstimationResultType: EnumFacialAgeEstimationResultType_INTERNAL,
	FacialAnimationStreamingState: EnumFacialAnimationStreamingState_INTERNAL,
	FacsActionUnit: EnumFacsActionUnit_INTERNAL,
	FeatureRestrictionAbuseVector: EnumFeatureRestrictionAbuseVector_INTERNAL,
	FeedbackType: EnumFeedbackType_INTERNAL,
	FieldOfViewMode: EnumFieldOfViewMode_INTERNAL,
	FillDirection: EnumFillDirection_INTERNAL,
	FilterErrorType: EnumFilterErrorType_INTERNAL,
	FilterResult: EnumFilterResult_INTERNAL,
	FilterType: EnumFilterType_INTERNAL,
	FinishRecordingOperation: EnumFinishRecordingOperation_INTERNAL,
	FluidFidelity: EnumFluidFidelity_INTERNAL,
	FluidForces: EnumFluidForces_INTERNAL,
	Font: EnumFont_INTERNAL,
	FontSize: EnumFontSize_INTERNAL,
	FontStyle: EnumFontStyle_INTERNAL,
	FontWeight: EnumFontWeight_INTERNAL,
	ForceLimitMode: EnumForceLimitMode_INTERNAL,
	FormFactor: EnumFormFactor_INTERNAL,
	FrameStyle: EnumFrameStyle_INTERNAL,
	FramerateManagerMode: EnumFramerateManagerMode_INTERNAL,
	FriendRequestEvent: EnumFriendRequestEvent_INTERNAL,
	FriendStatus: EnumFriendStatus_INTERNAL,
	FunctionalTestResult: EnumFunctionalTestResult_INTERNAL,
	GameAvatarType: EnumGameAvatarType_INTERNAL,
	GamepadType: EnumGamepadType_INTERNAL,
	GearGenreSetting: EnumGearGenreSetting_INTERNAL,
	GearType: EnumGearType_INTERNAL,
	Genre: EnumGenre_INTERNAL,
	GradientTileMode: EnumGradientTileMode_INTERNAL,
	GradientType: EnumGradientType_INTERNAL,
	GraphicsMode: EnumGraphicsMode_INTERNAL,
	GraphicsOptimizationMode: EnumGraphicsOptimizationMode_INTERNAL,
	GroupMembershipStatus: EnumGroupMembershipStatus_INTERNAL,
	GuiState: EnumGuiState_INTERNAL,
	GuiType: EnumGuiType_INTERNAL,
	HandlesStyle: EnumHandlesStyle_INTERNAL,
	HapticEffectType: EnumHapticEffectType_INTERNAL,
	HashAlgorithm: EnumHashAlgorithm_INTERNAL,
	HighlightDepthMode: EnumHighlightDepthMode_INTERNAL,
	HorizontalAlignment: EnumHorizontalAlignment_INTERNAL,
	HoverAnimateSpeed: EnumHoverAnimateSpeed_INTERNAL,
	HttpCachePolicy: EnumHttpCachePolicy_INTERNAL,
	HttpCompression: EnumHttpCompression_INTERNAL,
	HttpContentType: EnumHttpContentType_INTERNAL,
	HttpError: EnumHttpError_INTERNAL,
	HttpRequestType: EnumHttpRequestType_INTERNAL,
	HumanoidCollisionType: EnumHumanoidCollisionType_INTERNAL,
	HumanoidDisplayDistanceType: EnumHumanoidDisplayDistanceType_INTERNAL,
	HumanoidHealthDisplayType: EnumHumanoidHealthDisplayType_INTERNAL,
	HumanoidRigType: EnumHumanoidRigType_INTERNAL,
	HumanoidStateType: EnumHumanoidStateType_INTERNAL,
	IKCollisionsMode: EnumIKCollisionsMode_INTERNAL,
	IKControlConstraintSupport: EnumIKControlConstraintSupport_INTERNAL,
	IKControlType: EnumIKControlType_INTERNAL,
	IXPLoadingStatus: EnumIXPLoadingStatus_INTERNAL,
	ImageAlphaType: EnumImageAlphaType_INTERNAL,
	ImageCombineType: EnumImageCombineType_INTERNAL,
	InOut: EnumInOut_INTERNAL,
	InfoType: EnumInfoType_INTERNAL,
	InitialDockState: EnumInitialDockState_INTERNAL,
	InputActionType: EnumInputActionType_INTERNAL,
	InputBindingType: EnumInputBindingType_INTERNAL,
	InputSink: EnumInputSink_INTERNAL,
	InputType: EnumInputType_INTERNAL,
	InstanceFileSyncStatus: EnumInstanceFileSyncStatus_INTERNAL,
	IntermediateMeshGenerationResult: EnumIntermediateMeshGenerationResult_INTERNAL,
	InternalVideoUsage: EnumInternalVideoUsage_INTERNAL,
	InterpolationThrottlingMode: EnumInterpolationThrottlingMode_INTERNAL,
	InviteState: EnumInviteState_INTERNAL,
	ItemLineAlignment: EnumItemLineAlignment_INTERNAL,
	JoinSource: EnumJoinSource_INTERNAL,
	JointCreationMode: EnumJointCreationMode_INTERNAL,
	KeyCode: EnumKeyCode_INTERNAL,
	KeyCodeStringFormat: EnumKeyCodeStringFormat_INTERNAL,
	KeyInterpolationMode: EnumKeyInterpolationMode_INTERNAL,
	KeywordFilterType: EnumKeywordFilterType_INTERNAL,
	Language: EnumLanguage_INTERNAL,
	LeftRight: EnumLeftRight_INTERNAL,
	LexemeType: EnumLexemeType_INTERNAL,
	LightingStyle: EnumLightingStyle_INTERNAL,
	Limb: EnumLimb_INTERNAL,
	LineJoinMode: EnumLineJoinMode_INTERNAL,
	ListDisplayMode: EnumListDisplayMode_INTERNAL,
	ListenerLocation: EnumListenerLocation_INTERNAL,
	ListenerPositionType: EnumListenerPositionType_INTERNAL,
	ListenerType: EnumListenerType_INTERNAL,
	LiveEditingAtomicUpdateResponse: EnumLiveEditingAtomicUpdateResponse_INTERNAL,
	LiveEditingBroadcastMessageType: EnumLiveEditingBroadcastMessageType_INTERNAL,
	LoadCharacterLayeredClothing: EnumLoadCharacterLayeredClothing_INTERNAL,
	LoadDynamicHeads: EnumLoadDynamicHeads_INTERNAL,
	LocationType: EnumLocationType_INTERNAL,
	LuauTypeCheckMode: EnumLuauTypeCheckMode_INTERNAL,
	MakeupType: EnumMakeupType_INTERNAL,
	MarketplaceBulkPurchasePromptStatus: EnumMarketplaceBulkPurchasePromptStatus_INTERNAL,
	MarketplaceItemPurchaseStatus: EnumMarketplaceItemPurchaseStatus_INTERNAL,
	MarketplaceProductType: EnumMarketplaceProductType_INTERNAL,
	MarkupKind: EnumMarkupKind_INTERNAL,
	MatchmakingType: EnumMatchmakingType_INTERNAL,
	Material: EnumMaterial_INTERNAL,
	MaterialPattern: EnumMaterialPattern_INTERNAL,
	MembershipType: EnumMembershipType_INTERNAL,
	MeshPartDetailLevel: EnumMeshPartDetailLevel_INTERNAL,
	MeshPartHeadsAndAccessories: EnumMeshPartHeadsAndAccessories_INTERNAL,
	MeshScaleUnit: EnumMeshScaleUnit_INTERNAL,
	MeshType: EnumMeshType_INTERNAL,
	MessageType: EnumMessageType_INTERNAL,
	ModelLevelOfDetail: EnumModelLevelOfDetail_INTERNAL,
	ModelStreamingBehavior: EnumModelStreamingBehavior_INTERNAL,
	ModelStreamingMode: EnumModelStreamingMode_INTERNAL,
	ModerationResultCategory: EnumModerationResultCategory_INTERNAL,
	ModerationResultLabel: EnumModerationResultLabel_INTERNAL,
	ModerationStatus: EnumModerationStatus_INTERNAL,
	ModifierKey: EnumModifierKey_INTERNAL,
	MouseBehavior: EnumMouseBehavior_INTERNAL,
	MoveState: EnumMoveState_INTERNAL,
	MuteState: EnumMuteState_INTERNAL,
	NameOcclusion: EnumNameOcclusion_INTERNAL,
	NegateOperationHiddenHistory: EnumNegateOperationHiddenHistory_INTERNAL,
	NetworkOwnership: EnumNetworkOwnership_INTERNAL,
	NetworkStatus: EnumNetworkStatus_INTERNAL,
	NoiseType: EnumNoiseType_INTERNAL,
	NormalId: EnumNormalId_INTERNAL,
	NotificationButtonType: EnumNotificationButtonType_INTERNAL,
	OperationType: EnumOperationType_INTERNAL,
	OrientationAlignmentMode: EnumOrientationAlignmentMode_INTERNAL,
	OutfitSource: EnumOutfitSource_INTERNAL,
	OutfitType: EnumOutfitType_INTERNAL,
	OutputLayoutMode: EnumOutputLayoutMode_INTERNAL,
	OverrideMouseIconBehavior: EnumOverrideMouseIconBehavior_INTERNAL,
	PackagePermission: EnumPackagePermission_INTERNAL,
	PartType: EnumPartType_INTERNAL,
	ParticleEmitterShape: EnumParticleEmitterShape_INTERNAL,
	ParticleEmitterShapeInOut: EnumParticleEmitterShapeInOut_INTERNAL,
	ParticleEmitterShapeStyle: EnumParticleEmitterShapeStyle_INTERNAL,
	ParticleFlipbookLayout: EnumParticleFlipbookLayout_INTERNAL,
	ParticleFlipbookMode: EnumParticleFlipbookMode_INTERNAL,
	ParticleFlipbookTextureCompatible: EnumParticleFlipbookTextureCompatible_INTERNAL,
	ParticleOrientation: EnumParticleOrientation_INTERNAL,
	PathStatus: EnumPathStatus_INTERNAL,
	PathWaypointAction: EnumPathWaypointAction_INTERNAL,
	PathfindingUseImprovedSearch: EnumPathfindingUseImprovedSearch_INTERNAL,
	PeoplePageLayout: EnumPeoplePageLayout_INTERNAL,
	PerformanceOverlayMode: EnumPerformanceOverlayMode_INTERNAL,
	PermissionLevelShown: EnumPermissionLevelShown_INTERNAL,
	PhysicalConstraintType: EnumPhysicalConstraintType_INTERNAL,
	PhysicsSimulationRate: EnumPhysicsSimulationRate_INTERNAL,
	PhysicsSteppingMethod: EnumPhysicsSteppingMethod_INTERNAL,
	PlaceContentPreference: EnumPlaceContentPreference_INTERNAL,
	PlacePublishType: EnumPlacePublishType_INTERNAL,
	Platform: EnumPlatform_INTERNAL,
	PlaybackState: EnumPlaybackState_INTERNAL,
	PlayerActions: EnumPlayerActions_INTERNAL,
	PlayerCharacterDestroyBehavior: EnumPlayerCharacterDestroyBehavior_INTERNAL,
	PlayerChatType: EnumPlayerChatType_INTERNAL,
	PlayerDataErrorState: EnumPlayerDataErrorState_INTERNAL,
	PlayerDataLoadFailureBehavior: EnumPlayerDataLoadFailureBehavior_INTERNAL,
	PlayerExitReason: EnumPlayerExitReason_INTERNAL,
	PlayerPlatformActivationStatus: EnumPlayerPlatformActivationStatus_INTERNAL,
	PlayerPlatformSpenderStatus: EnumPlayerPlatformSpenderStatus_INTERNAL,
	PluginConnectionTargetType: EnumPluginConnectionTargetType_INTERNAL,
	PoseEasingDirection: EnumPoseEasingDirection_INTERNAL,
	PoseEasingStyle: EnumPoseEasingStyle_INTERNAL,
	PositionAlignmentMode: EnumPositionAlignmentMode_INTERNAL,
	PredictionMode: EnumPredictionMode_INTERNAL,
	PredictionStatus: EnumPredictionStatus_INTERNAL,
	PreferredInput: EnumPreferredInput_INTERNAL,
	PreferredTextSize: EnumPreferredTextSize_INTERNAL,
	PrefetchDownloadStatus: EnumPrefetchDownloadStatus_INTERNAL,
	PrimalPhysicsSolver: EnumPrimalPhysicsSolver_INTERNAL,
	PrimitiveType: EnumPrimitiveType_INTERNAL,
	PrivilegeType: EnumPrivilegeType_INTERNAL,
	ProductLocationRestriction: EnumProductLocationRestriction_INTERNAL,
	ProductPurchaseChannel: EnumProductPurchaseChannel_INTERNAL,
	ProductPurchaseDecision: EnumProductPurchaseDecision_INTERNAL,
	PromptCreateAssetResult: EnumPromptCreateAssetResult_INTERNAL,
	PromptCreateAvatarResult: EnumPromptCreateAvatarResult_INTERNAL,
	PromptExperienceDetailsResult: EnumPromptExperienceDetailsResult_INTERNAL,
	PromptLinkSharingResult: EnumPromptLinkSharingResult_INTERNAL,
	PromptPublishAssetResult: EnumPromptPublishAssetResult_INTERNAL,
	PropertyStatus: EnumPropertyStatus_INTERNAL,
	ProximityPromptExclusivity: EnumProximityPromptExclusivity_INTERNAL,
	ProximityPromptInputType: EnumProximityPromptInputType_INTERNAL,
	ProximityPromptStyle: EnumProximityPromptStyle_INTERNAL,
	PurchaseOption: EnumPurchaseOption_INTERNAL,
	QualityLevel: EnumQualityLevel_INTERNAL,
	R15CollisionType: EnumR15CollisionType_INTERNAL,
	RaycastFilterType: EnumRaycastFilterType_INTERNAL,
	ReadCapturesFromGalleryResult: EnumReadCapturesFromGalleryResult_INTERNAL,
	ReceiptDecision: EnumReceiptDecision_INTERNAL,
	ReceiptType: EnumReceiptType_INTERNAL,
	RecommendationActionType: EnumRecommendationActionType_INTERNAL,
	RecommendationDepartureIntent: EnumRecommendationDepartureIntent_INTERNAL,
	RecommendationImpressionType: EnumRecommendationImpressionType_INTERNAL,
	RecommendationItemContentType: EnumRecommendationItemContentType_INTERNAL,
	RecommendationItemVisibility: EnumRecommendationItemVisibility_INTERNAL,
	RecommendationPreferenceTargetType: EnumRecommendationPreferenceTargetType_INTERNAL,
	RecommendationPreferenceType: EnumRecommendationPreferenceType_INTERNAL,
	RejectCharacterDeletions: EnumRejectCharacterDeletions_INTERNAL,
	RenderFidelity: EnumRenderFidelity_INTERNAL,
	RenderPriority: EnumRenderPriority_INTERNAL,
	RenderingCacheOptimizationMode: EnumRenderingCacheOptimizationMode_INTERNAL,
	RenderingTestComparisonMethod: EnumRenderingTestComparisonMethod_INTERNAL,
	ReplicateInstanceDestroySetting: EnumReplicateInstanceDestroySetting_INTERNAL,
	ResamplerMode: EnumResamplerMode_INTERNAL,
	ReservedHighlightId: EnumReservedHighlightId_INTERNAL,
	RestPose: EnumRestPose_INTERNAL,
	RestPoseModel: EnumRestPoseModel_INTERNAL,
	ReturnKeyType: EnumReturnKeyType_INTERNAL,
	ReverbType: EnumReverbType_INTERNAL,
	ReviewableContentState: EnumReviewableContentState_INTERNAL,
	RibbonTool: EnumRibbonTool_INTERNAL,
	RigLabel: EnumRigLabel_INTERNAL,
	RigScale: EnumRigScale_INTERNAL,
	RigType: EnumRigType_INTERNAL,
	RollOffMode: EnumRollOffMode_INTERNAL,
	RolloutState: EnumRolloutState_INTERNAL,
	RotationOrder: EnumRotationOrder_INTERNAL,
	RotationType: EnumRotationType_INTERNAL,
	RsvpStatus: EnumRsvpStatus_INTERNAL,
	RtlTextSupport: EnumRtlTextSupport_INTERNAL,
	RunContext: EnumRunContext_INTERNAL,
	RunState: EnumRunState_INTERNAL,
	RuntimeUndoBehavior: EnumRuntimeUndoBehavior_INTERNAL,
	SafeAreaCompatibility: EnumSafeAreaCompatibility_INTERNAL,
	SalesTypeFilter: EnumSalesTypeFilter_INTERNAL,
	SandboxedInstanceMode: EnumSandboxedInstanceMode_INTERNAL,
	SaveAvatarThumbnailCustomizationFailure: EnumSaveAvatarThumbnailCustomizationFailure_INTERNAL,
	SaveFilter: EnumSaveFilter_INTERNAL,
	SavedQualitySetting: EnumSavedQualitySetting_INTERNAL,
	ScaleType: EnumScaleType_INTERNAL,
	ScopeCheckResult: EnumScopeCheckResult_INTERNAL,
	ScreenInsets: EnumScreenInsets_INTERNAL,
	ScreenOrientation: EnumScreenOrientation_INTERNAL,
	ScreenshotCaptureResult: EnumScreenshotCaptureResult_INTERNAL,
	ScriptStoppedReason: EnumScriptStoppedReason_INTERNAL,
	ScriptVariableScope: EnumScriptVariableScope_INTERNAL,
	ScrollBarInset: EnumScrollBarInset_INTERNAL,
	ScrollingDirection: EnumScrollingDirection_INTERNAL,
	SecurityCapability: EnumSecurityCapability_INTERNAL,
	SelectionBehavior: EnumSelectionBehavior_INTERNAL,
	SelectionRenderMode: EnumSelectionRenderMode_INTERNAL,
	SelfViewPosition: EnumSelfViewPosition_INTERNAL,
	SensorMode: EnumSensorMode_INTERNAL,
	SensorUpdateType: EnumSensorUpdateType_INTERNAL,
	ServerLiveEditingMode: EnumServerLiveEditingMode_INTERNAL,
	ServiceVisibility: EnumServiceVisibility_INTERNAL,
	Severity: EnumSeverity_INTERNAL,
	ShowAdResult: EnumShowAdResult_INTERNAL,
	SignalBehavior: EnumSignalBehavior_INTERNAL,
	SimulationMode: EnumSimulationMode_INTERNAL,
	SizeConstraint: EnumSizeConstraint_INTERNAL,
	SlimTintMode: EnumSlimTintMode_INTERNAL,
	SolidPrimitiveType: EnumSolidPrimitiveType_INTERNAL,
	SolverConvergenceMetricType: EnumSolverConvergenceMetricType_INTERNAL,
	SolverConvergenceVisualizationMode: EnumSolverConvergenceVisualizationMode_INTERNAL,
	SortDirection: EnumSortDirection_INTERNAL,
	SortOrder: EnumSortOrder_INTERNAL,
	SpecialKey: EnumSpecialKey_INTERNAL,
	StartCorner: EnumStartCorner_INTERNAL,
	StateObjectFieldType: EnumStateObjectFieldType_INTERNAL,
	Status: EnumStatus_INTERNAL,
	StepFrequency: EnumStepFrequency_INTERNAL,
	StreamOutBehavior: EnumStreamOutBehavior_INTERNAL,
	StreamingIntegrityMode: EnumStreamingIntegrityMode_INTERNAL,
	StreamingPauseMode: EnumStreamingPauseMode_INTERNAL,
	StrokeSizingMode: EnumStrokeSizingMode_INTERNAL,
	StudioAction: EnumStudioAction_INTERNAL,
	StudioCaptureBufferStatus: EnumStudioCaptureBufferStatus_INTERNAL,
	StudioCaptureScreenshotFormat: EnumStudioCaptureScreenshotFormat_INTERNAL,
	StudioCloseMode: EnumStudioCloseMode_INTERNAL,
	StudioDataModelType: EnumStudioDataModelType_INTERNAL,
	StudioPlaceUpdateFailureReason: EnumStudioPlaceUpdateFailureReason_INTERNAL,
	StudioScriptEditorColorCategories: EnumStudioScriptEditorColorCategories_INTERNAL,
	StudioScriptEditorColorPresets: EnumStudioScriptEditorColorPresets_INTERNAL,
	StudioStyleGuideColor: EnumStudioStyleGuideColor_INTERNAL,
	StudioStyleGuideModifier: EnumStudioStyleGuideModifier_INTERNAL,
	Style: EnumStyle_INTERNAL,
	SubscriptionExpirationReason: EnumSubscriptionExpirationReason_INTERNAL,
	SubscriptionPaymentStatus: EnumSubscriptionPaymentStatus_INTERNAL,
	SubscriptionPeriod: EnumSubscriptionPeriod_INTERNAL,
	SubscriptionState: EnumSubscriptionState_INTERNAL,
	SurfaceConstraint: EnumSurfaceConstraint_INTERNAL,
	SurfaceGuiShape: EnumSurfaceGuiShape_INTERNAL,
	SurfaceGuiSizingMode: EnumSurfaceGuiSizingMode_INTERNAL,
	SurfaceType: EnumSurfaceType_INTERNAL,
	SwipeDirection: EnumSwipeDirection_INTERNAL,
	SystemThemeValue: EnumSystemThemeValue_INTERNAL,
	TableMajorAxis: EnumTableMajorAxis_INTERNAL,
	TagReplicability: EnumTagReplicability_INTERNAL,
	TeamCreateErrorState: EnumTeamCreateErrorState_INTERNAL,
	Technology: EnumTechnology_INTERNAL,
	TelemetryBackend: EnumTelemetryBackend_INTERNAL,
	TelemetryStandardizedField: EnumTelemetryStandardizedField_INTERNAL,
	TeleportMethod: EnumTeleportMethod_INTERNAL,
	TeleportResult: EnumTeleportResult_INTERNAL,
	TeleportState: EnumTeleportState_INTERNAL,
	TeleportType: EnumTeleportType_INTERNAL,
	TerrainAcquisitionMethod: EnumTerrainAcquisitionMethod_INTERNAL,
	TerrainFace: EnumTerrainFace_INTERNAL,
	TerrainLiquidMergeOperation: EnumTerrainLiquidMergeOperation_INTERNAL,
	TerrainSolidMergeOperation: EnumTerrainSolidMergeOperation_INTERNAL,
	TextChatMessageStatus: EnumTextChatMessageStatus_INTERNAL,
	TextDirection: EnumTextDirection_INTERNAL,
	TextFilterContext: EnumTextFilterContext_INTERNAL,
	TextInputType: EnumTextInputType_INTERNAL,
	TextTruncate: EnumTextTruncate_INTERNAL,
	TextXAlignment: EnumTextXAlignment_INTERNAL,
	TextYAlignment: EnumTextYAlignment_INTERNAL,
	TextureMode: EnumTextureMode_INTERNAL,
	TextureQueryType: EnumTextureQueryType_INTERNAL,
	ThreadPoolConfig: EnumThreadPoolConfig_INTERNAL,
	ThrottlingPriority: EnumThrottlingPriority_INTERNAL,
	ThumbnailSize: EnumThumbnailSize_INTERNAL,
	ThumbnailType: EnumThumbnailType_INTERNAL,
	TickCountSampleMethod: EnumTickCountSampleMethod_INTERNAL,
	TonemapperPreset: EnumTonemapperPreset_INTERNAL,
	TopBottom: EnumTopBottom_INTERNAL,
	TouchCameraMovementMode: EnumTouchCameraMovementMode_INTERNAL,
	TouchMovementMode: EnumTouchMovementMode_INTERNAL,
	TrackerError: EnumTrackerError_INTERNAL,
	TrackerExtrapolationFlagMode: EnumTrackerExtrapolationFlagMode_INTERNAL,
	TrackerFaceTrackingStatus: EnumTrackerFaceTrackingStatus_INTERNAL,
	TrackerLodFlagMode: EnumTrackerLodFlagMode_INTERNAL,
	TrackerLodValueMode: EnumTrackerLodValueMode_INTERNAL,
	TrackerMode: EnumTrackerMode_INTERNAL,
	TrackerPromptEvent: EnumTrackerPromptEvent_INTERNAL,
	TrackerType: EnumTrackerType_INTERNAL,
	TriStateBoolean: EnumTriStateBoolean_INTERNAL,
	TweenStatus: EnumTweenStatus_INTERNAL,
	UICaptureMode: EnumUICaptureMode_INTERNAL,
	UIDragDetectorBoundingBehavior: EnumUIDragDetectorBoundingBehavior_INTERNAL,
	UIDragDetectorDragRelativity: EnumUIDragDetectorDragRelativity_INTERNAL,
	UIDragDetectorDragSpace: EnumUIDragDetectorDragSpace_INTERNAL,
	UIDragDetectorDragStyle: EnumUIDragDetectorDragStyle_INTERNAL,
	UIDragDetectorResponseStyle: EnumUIDragDetectorResponseStyle_INTERNAL,
	UIDragSpeedAxisMapping: EnumUIDragSpeedAxisMapping_INTERNAL,
	UIFlexAlignment: EnumUIFlexAlignment_INTERNAL,
	UIFlexMode: EnumUIFlexMode_INTERNAL,
	UITheme: EnumUITheme_INTERNAL,
	UiMessageType: EnumUiMessageType_INTERNAL,
	UpdateState: EnumUpdateState_INTERNAL,
	UploadCaptureResult: EnumUploadCaptureResult_INTERNAL,
	UsageContext: EnumUsageContext_INTERNAL,
	UserCFrame: EnumUserCFrame_INTERNAL,
	UserInputState: EnumUserInputState_INTERNAL,
	UserInputType: EnumUserInputType_INTERNAL,
	VRComfortSetting: EnumVRComfortSetting_INTERNAL,
	VRControllerModelMode: EnumVRControllerModelMode_INTERNAL,
	VRDeviceType: EnumVRDeviceType_INTERNAL,
	VRLaserPointerMode: EnumVRLaserPointerMode_INTERNAL,
	VRSafetyBubbleMode: EnumVRSafetyBubbleMode_INTERNAL,
	VRScaling: EnumVRScaling_INTERNAL,
	VRSessionState: EnumVRSessionState_INTERNAL,
	VRTouchpad: EnumVRTouchpad_INTERNAL,
	VRTouchpadMode: EnumVRTouchpadMode_INTERNAL,
	VelocityConstraintMode: EnumVelocityConstraintMode_INTERNAL,
	VerticalAlignment: EnumVerticalAlignment_INTERNAL,
	VerticalScrollBarPosition: EnumVerticalScrollBarPosition_INTERNAL,
	VibrationMotor: EnumVibrationMotor_INTERNAL,
	VideoCaptureResult: EnumVideoCaptureResult_INTERNAL,
	VideoCaptureStartedResult: EnumVideoCaptureStartedResult_INTERNAL,
	VideoDeviceCaptureQuality: EnumVideoDeviceCaptureQuality_INTERNAL,
	VideoError: EnumVideoError_INTERNAL,
	VideoSampleSize: EnumVideoSampleSize_INTERNAL,
	ViewMode: EnumViewMode_INTERNAL,
	VirtualCursorMode: EnumVirtualCursorMode_INTERNAL,
	VirtualInputMode: EnumVirtualInputMode_INTERNAL,
	VoiceChatDistanceAttenuationType: EnumVoiceChatDistanceAttenuationType_INTERNAL,
	VoiceChatState: EnumVoiceChatState_INTERNAL,
	VoiceClientLeaveReasons: EnumVoiceClientLeaveReasons_INTERNAL,
	VoiceControlPath: EnumVoiceControlPath_INTERNAL,
	VoiceRccReconnectReason: EnumVoiceRccReconnectReason_INTERNAL,
	VolumetricAudio: EnumVolumetricAudio_INTERNAL,
	WaterDirection: EnumWaterDirection_INTERNAL,
	WaterForce: EnumWaterForce_INTERNAL,
	WebSocketState: EnumWebSocketState_INTERNAL,
	WebStreamClientState: EnumWebStreamClientState_INTERNAL,
	WebStreamClientType: EnumWebStreamClientType_INTERNAL,
	WeldConstraintPreserve: EnumWeldConstraintPreserve_INTERNAL,
	WhenUserFirstPlayed: EnumWhenUserFirstPlayed_INTERNAL,
	WhisperChatPrivacyMode: EnumWhisperChatPrivacyMode_INTERNAL,
	WrapLayerAutoSkin: EnumWrapLayerAutoSkin_INTERNAL,
	WrapLayerDebugMode: EnumWrapLayerDebugMode_INTERNAL,
	WrapTargetDebugMode: EnumWrapTargetDebugMode_INTERNAL,
	ZIndexBehavior: EnumZIndexBehavior_INTERNAL,
}
