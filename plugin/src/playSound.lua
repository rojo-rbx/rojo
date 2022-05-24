-- Roblox decided that sounds only play in Edit mode when parented to a plugin widget, for some reason
local plugin = plugin or script:FindFirstAncestorWhichIsA("Plugin")
local widget = plugin:CreateDockWidgetPluginGui("Rojo_soundPlayer", DockWidgetPluginGuiInfo.new(
	Enum.InitialDockState.Float,
	false, true,
	10, 10,
	10, 10
))
widget.Name = "Rojo_soundPlayer"
widget.Title = "Rojo Sound Player"

return function(soundId)
	local sound = Instance.new("Sound")
	sound.SoundId = soundId
	sound.Parent = widget

	sound.Ended:Connect(function()
		sound:Destroy()
	end)

	sound:Play()
end
