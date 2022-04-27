--[[
	Roblox plugins have painfully limited audio capabilities.
	https://devforum.roblox.com/t/allow-sounds-to-be-played-in-studio/403644
--]]

local SoundService = game:GetService("SoundService")

return function(soundId)
	local sound = Instance.new("Sound")
	sound.SoundId = soundId

	sound.Ended:Connect(function()
		sound:Destroy()
	end)

	SoundService:PlayLocalSound(sound)
end
