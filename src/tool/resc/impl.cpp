Sexy::Image* LoadImageById(Sexy::ResourceManager* manager, ResourceId id) {
    const char* name = IdToString(id);
    if (!name || name[0] == '\0') {
        return nullptr;
    }
    auto intId = static_cast<int>(id);
    gResources[intId] = manager->LoadImage(name);
    return reinterpret_cast<Sexy::Image*>(gResources[intId]);
}
void ReplaceImageById(Sexy::ResourceManager* manager, ResourceId id, Sexy::Image* image) {
    const char* name = IdToString(id);
    if (!name || name[0] == '\0') {
        return;
    }
    manager->ReplaceImage(name, image);
    auto intId = static_cast<int>(id);
    gResources[intId] = image;
}
namespace {
template<typename T>
T GetResourceById(ResourceId id) {
    auto intId = static_cast<int>(id);
    if (intId >= static_cast<int>(ResourceId::LENGTH)) {
        return nullptr;
    }
    return reinterpret_cast<T>(gResources[intId]);
}
}
Sexy::Image* GetImageById(ResourceId id) {
    return GetResourceById<Sexy::Image*>(id);
}
Sexy::Font* GetFontById(ResourceId id) {
    return GetResourceById<Sexy::Font*>(id);
}
int GetSoundById(ResourceId id) {
    return GetResourceById<int>(id);
}
