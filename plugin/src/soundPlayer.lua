-- Sounds only play in Edit mode when parented to a plugin widget, for some reason
local plugin = plugin or script:FindFirstAncestorWhichIsA("Plugin")
local widget = nil
if plugin then
	widget = plugin:CreateDockWidgetPluginGui("Rojo_soundPlayer", DockWidgetPluginGuiInfo.new(
		Enum.InitialDockState.Float,
		false, true,
		10, 10,
		10, 10
	))
	widget.Name = "Rojo_soundPlayer"
	widget.Title = "Rojo Sound Player"
end

local SoundPlayer = {}
SoundPlayer.__index = SoundPlayer

function SoundPlayer.new(settings)
	return setmetatable({
		settings = settings,
	}, SoundPlayer)
end

function SoundPlayer:play(soundId)
	if self.settings and self.settings:get("playSounds") == false then return end

	local sound = Instance.new("Sound")
	sound.SoundId = soundId
	sound.Parent = widget

	sound.Ended:Connect(function()
		sound:Destroy()
	end)

	sound:Play()
end

return SoundPlayer
