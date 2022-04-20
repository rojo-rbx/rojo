return function()
    local setAttribute = require(script.Parent.setAttribute)
    local inst = Instance.new("Folder")

    it("should accept supported attribute types", function()
        local values = {
            "Test",

            true,
            1337.9001,

            UDim.new(1, 2),
            UDim2.new(3, 4, 5, 6),

            BrickColor.Red(),
            Color3.new(),

            Vector2.new(),
            Vector3.new(),

            ColorSequence.new(Color3.new()),
            NumberSequence.new(0),

            NumberRange.new(0),
            Rect.new(),
        }

        for _, value in ipairs(values) do
            local set = setAttribute(inst, "Test", value)
            expect(set).to.be.ok()

            local read = inst:GetAttribute("Test")
            expect(read).to.equal(value)
        end
    end)

    it("should not accept names that start with RBX", function()
        local set = setAttribute(inst, "RBXTest", 0)
        expect(set).to.never.be.ok()
    end)

    it("should not accept names outside the range of [1,100] characters.", function()
        local tooMany = string.rep("A", 101)
        local tooFew = ""

        local set = setAttribute(inst, tooMany, 0)
        expect(set).to.never.be.ok()

        set = setAttribute(inst, tooFew, 0)
        expect(set).to.never.be.ok()
    end)

    it("should not accept names if characters aren't alphanumeric/underscores", function()
        local badNames = {
            "no spaces",
            "  bad",
            "mad  ",
            " pad ",
            "#####",
            "?.:@/~",
        }

        for _, badName in ipairs(badNames) do
            local set = setAttribute(inst, badName, 0)
            expect(set).to.never.be.ok()
        end
    end)

    it("should accept names that start with numbers or underscores", function()
        local coolNames = {
            "___",
            "1_3_3_7",
            "_neat",
            "__index",
            "_epic_",
        }

        for _, coolName in ipairs(coolNames) do
            local set = setAttribute(inst, coolName, 0)
            expect(set).to.be.ok()
        end
    end)
end