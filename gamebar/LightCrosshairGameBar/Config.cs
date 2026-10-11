using System;
using Windows.Storage;

namespace LightCrosshairGameBar
{
    sealed class Config
    {
        readonly ApplicationDataContainer values = ApplicationData.Current.LocalSettings;
        public static event Action Changed;
        public string ImageToken { get => Get("imageToken", ""); set => Set("imageToken", value); }
        public bool TintEnabled { get => Get("tintEnabled", false); set => Set("tintEnabled", value); }
        public string Tint { get => Get("tint", "#00FF00"); set => Set("tint", value); }
        public double Scale { get => Get("scale", 1d); set => Set("scale", Math.Max(0, Math.Min(5, value))); }
        public double Opacity { get => Get("opacity", 1d); set => Set("opacity", Math.Max(0, Math.Min(1, value))); }
        public double OffsetX { get => Get("offsetX", 0d); set => Set("offsetX", Math.Max(-100, Math.Min(100, value))); }
        public double OffsetY { get => Get("offsetY", 0d); set => Set("offsetY", Math.Max(-100, Math.Min(100, value))); }
        public bool Visible { get => Get("visible", true); set => Set("visible", value); }
        public string CenterHotkey { get => Get("centerHotkey", "Ctrl+Alt+C"); set => Set("centerHotkey", value); }
        public string HideHotkey { get => Get("hideHotkey", "Ctrl+Alt+H"); set => Set("hideHotkey", value); }
        public string ShowHotkey { get => Get("showHotkey", "Ctrl+Alt+S"); set => Set("showHotkey", value); }
        T Get<T>(string key, T fallback) => values.Values.TryGetValue(key, out var value) && value is T typed ? typed : fallback;
        void Set(string key, object value) { values.Values[key] = value; Changed?.Invoke(); }
    }
}
