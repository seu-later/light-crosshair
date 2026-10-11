using System;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;

namespace LightCrosshairGameBar
{
    public sealed partial class SettingsPage : Page
    {
        readonly Config config = new Config(); bool loading;
        public SettingsPage() { InitializeComponent(); Load(); }
        void Load() { loading = true; TintEnabled.IsChecked = config.TintEnabled; Tint.Text = config.Tint; ScaleBox.Text = config.Scale.ToString("0.00"); OpacityBox.Text = (config.Opacity * 100).ToString("0"); OffsetX.Text = config.OffsetX.ToString("0"); OffsetY.Text = config.OffsetY.ToString("0"); CenterHotkey.Text = config.CenterHotkey; HideHotkey.Text = config.HideHotkey; ShowHotkey.Text = config.ShowHotkey; ImageName.Text = string.IsNullOrEmpty(config.ImageToken) ? "No image selected" : "PNG selected"; loading = false; }
        async void LoadImage_Click(object sender, RoutedEventArgs e) { var picker = new FileOpenPicker(); picker.FileTypeFilter.Add(".png"); var file = await picker.PickSingleFileAsync(); if (file == null) return; config.ImageToken = StorageApplicationPermissions.FutureAccessList.Add(file); ImageName.Text = file.Name; }
        void Changed(object sender, object e) { if (loading) return; config.TintEnabled = TintEnabled.IsChecked == true; config.Tint = Tint.Text; config.CenterHotkey = CenterHotkey.Text; config.HideHotkey = HideHotkey.Text; config.ShowHotkey = ShowHotkey.Text; if (double.TryParse(ScaleBox.Text, out var scale)) config.Scale = scale; if (double.TryParse(OpacityBox.Text, out var opacity)) config.Opacity = opacity / 100; if (double.TryParse(OffsetX.Text, out var offsetX)) config.OffsetX = offsetX; if (double.TryParse(OffsetY.Text, out var offsetY)) config.OffsetY = offsetY; }
    }
}
