Sexy::Image* LoadImageById(Sexy::ResourceManager* manager, ResourceId id) {
    const char* name = IdToString(id);
    if (!name || name[0] == '\0') {
        return nullptr;
    }
    auto intId = static_cast<int>(id);
    Sexy::Image* image = manager->LoadImage(name);
    Sexy::Image** ppImage = reinterpret_cast<Sexy::Image**>(gResources[intId]);
    *ppImage = image;
    return image;
}

namespace {
template <typename T>
T GetResourceById(ResourceId id, T fallback) {
    auto intId = static_cast<int>(id);
    if (intId >= static_cast<int>(ResourceId::LENGTH)) {
        return fallback;
    }
    T* pp = reinterpret_cast<T*>(gResources[intId]);
    return *pp;
}
}

Sexy::Image* GetImageById(ResourceId id) {
    return GetResourceById<Sexy::Image*>(id, nullptr);
}
Sexy::Font* GetFontById(ResourceId id) {
    return GetResourceById<Sexy::Font*>(id, nullptr);
}
int GetSoundById(ResourceId id) {
    return GetResourceById<int>(id, -1);
}

static std::atomic<bool> gPtrToIdMapDirty = true;
static std::mutex gPtrToIdMapMutex;
namespace {

bool IsImageId(ResourceId id) {
    return static_cast<int>(id) >= static_cast<int>(ResourceId::_IMAGE_FIRST)
        && static_cast<int>(id) <= static_cast<int>(ResourceId::_IMAGE_LAST);
}
bool IsFontId(ResourceId id) {
    return static_cast<int>(id) >= static_cast<int>(ResourceId::_FONT_FIRST)
        && static_cast<int>(id) <= static_cast<int>(ResourceId::_FONT_LAST);
}
bool IsSoundId(ResourceId id) {
    return static_cast<int>(id) >= static_cast<int>(ResourceId::_SOUND_FIRST)
        && static_cast<int>(id) <= static_cast<int>(ResourceId::_SOUND_LAST);
}

ResourceId GetIdByPtrKey(uintptr_t ptr) {
    static std::unordered_map<uintptr_t, ResourceId> gPtrToIdMap;
    if (gPtrToIdMapDirty) {
        auto lock = std::scoped_lock(gPtrToIdMapMutex);
        gPtrToIdMap.clear();
        int length = static_cast<int>(ResourceId::LENGTH);
        for (int i = 0; i < length; i++) {
            ResourceId iid = static_cast<ResourceId>(i);
            if (IsFontId(iid)) {
                auto key = reinterpret_cast<uintptr_t>(GetFontById(iid));
                gPtrToIdMap[key] = iid;
            } else if (IsImageId(iid)) {
                auto key = reinterpret_cast<uintptr_t>(GetImageById(iid));
                gPtrToIdMap[key] = iid;
            } else if (IsSoundId(iid)) {
                auto key = static_cast<uintptr_t>(GetSoundById(iid));
                gPtrToIdMap[key] = iid;
            }
        }
        gPtrToIdMapDirty = false;
    }
    auto it = gPtrToIdMap.find(ptr);
    if (it == gPtrToIdMap.end()) {
        return ResourceId::LENGTH;
    }
    return it->second;
}

}

ResourceId GetIdByImage(Sexy::Image* theImage) {
    if (!theImage) {
        return ResourceId::LENGTH;
    }
    return GetIdByPtrKey(reinterpret_cast<uintptr_t>(theImage));
}
ResourceId GetIdByFont(Sexy::Font* theFont) {
    if (!theFont) {
        return ResourceId::LENGTH;
    }
    return GetIdByPtrKey(reinterpret_cast<uintptr_t>(theFont));
}
ResourceId GetIdBySound(int theSound) {
    return GetIdByPtrKey(static_cast<uintptr_t>(theSound));
}
