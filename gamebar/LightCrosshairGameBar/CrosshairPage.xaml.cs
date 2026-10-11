using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.Gaming.XboxGameBar;
using Microsoft.Gaming.XboxGameBar.Input;
using Windows.Graphics.Imaging;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Streams;
using Windows.System;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;
using Windows.UI.Xaml.Media.Imaging;
using Windows.UI.Xaml.Navigation;

namespace LightCrosshairGameBar
{
    public sealed partial class CrosshairPage : Page
    {
        readonly Config config = new Config();
        XboxGameBarWidget widget;
        XboxGameBarHotkeyWatcher center, hide, show;
        uint imageWidth, imageHeight;

        public CrosshairPage()
        {
            InitializeComponent();
            Config.Changed += Config_Changed;
        }

        async void Config_Changed()
        {
            await Dispatcher.RunAsync(Windows.UI.Core.CoreDispatcherPriority.Normal, () => { ConfigureHotkeys(); ApplyAsync(); });
        }

        protected override void OnNavigatedTo(NavigationEventArgs e)
        {
            widget = (XboxGameBarWidget)e.Parameter;
            widget.SettingsClicked += Widget_SettingsClicked;
            widget.PinningSupported = true; widget.SettingsSupported = true;
            ConfigureHotkeys(); ApplyAsync();
        }

        void ConfigureHotkeys()
        {
            center?.Stop(); hide?.Stop(); show?.Stop();
            center = hide = show = null;
            if (TryHotkey(config.CenterHotkey, out var centerKeys)) { center = Watch(centerKeys); center.HotkeySetStateChanged += async (s, e) => { if (e.HotkeySetDown) await widget.CenterWindowAsync(); }; center.Start(); }
            if (TryHotkey(config.HideHotkey, out var hideKeys)) { hide = Watch(hideKeys); hide.HotkeySetStateChanged += (s, e) => { if (e.HotkeySetDown) SetVisible(false); }; hide.Start(); }
            if (TryHotkey(config.ShowHotkey, out var showKeys)) { show = Watch(showKeys); show.HotkeySetStateChanged += (s, e) => { if (e.HotkeySetDown) SetVisible(true); }; show.Start(); }
        }

        async void Widget_SettingsClicked(XboxGameBarWidget sender, object args) => await sender.ActivateSettingsAsync();

        XboxGameBarHotkeyWatcher Watch(IEnumerable<VirtualKey> keys) => XboxGameBarHotkeyWatcher.CreateWatcher(widget, new List<VirtualKey>(keys));

        static bool TryHotkey(string text, out List<VirtualKey> keys)
        {
            keys = new List<VirtualKey>();
            foreach (var part in (text ?? "").Split('+'))
            {
                var key = part.Trim();
                if (key.Equals("Ctrl", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.Control);
                else if (key.Equals("Alt", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.Menu);
                else if (key.Equals("Win", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.LeftWindows);
                else if (key.Equals("Left Click", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.LeftButton);
                else if (key.Equals("Right Click", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.RightButton);
                else if (key.Equals("Middle Click", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.MiddleButton);
                else if (key.Equals("Mouse 4", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.XButton1);
                else if (key.Equals("Mouse 5", StringComparison.OrdinalIgnoreCase)) keys.Add(VirtualKey.XButton2);
                else if (!Enum.TryParse(key, true, out VirtualKey parsed)) return false;
                else keys.Add(parsed);
            }
            return keys.Count > 0;
        }

        void SetVisible(bool visible) { config.Visible = visible; Crosshair.Visibility = visible ? Visibility.Visible : Visibility.Collapsed; }

        async void ApplyAsync()
        {
            Crosshair.Opacity = config.Opacity;
            if (!config.Visible || config.Scale == 0 || string.IsNullOrEmpty(config.ImageToken)) { Crosshair.Visibility = Visibility.Collapsed; return; }
            try
            {
                var file = await StorageApplicationPermissions.FutureAccessList.GetFileAsync(config.ImageToken);
                using (var stream = await file.OpenReadAsync())
                {
                    var decoder = await BitmapDecoder.CreateAsync(stream);
                    var pixels = (await decoder.GetPixelDataAsync(BitmapPixelFormat.Rgba8, BitmapAlphaMode.Straight, new BitmapTransform(), ExifOrientationMode.IgnoreExifOrientation, ColorManagementMode.DoNotColorManage)).DetachPixelData();
                    imageWidth = decoder.PixelWidth; imageHeight = decoder.PixelHeight;
                    if (imageWidth == 0 || imageHeight == 0 || imageWidth > 4096 || imageHeight > 4096) throw new InvalidDataException();
                    if (config.TintEnabled && !ValidHex(config.Tint)) throw new InvalidDataException();
                    var output = new byte[pixels.Length];
                    for (var i = 0; i < pixels.Length; i += 4)
                    {
                        var alpha = pixels[i + 3]; var red = pixels[i]; var green = pixels[i + 1]; var blue = pixels[i + 2];
                        if (config.TintEnabled) { red = (byte)((red * ParseHex(config.Tint, 1) + 127) / 255); green = (byte)((green * ParseHex(config.Tint, 3) + 127) / 255); blue = (byte)((blue * ParseHex(config.Tint, 5) + 127) / 255); }
                        output[i] = (byte)((blue * alpha + 127) / 255); output[i + 1] = (byte)((green * alpha + 127) / 255); output[i + 2] = (byte)((red * alpha + 127) / 255); output[i + 3] = alpha;
                    }
                    var bitmap = new WriteableBitmap((int)imageWidth, (int)imageHeight);
                    using (var destination = bitmap.PixelBuffer.AsStream()) destination.Write(output, 0, output.Length);
                    Crosshair.Source = bitmap; Crosshair.Width = imageWidth * config.Scale; Crosshair.Height = imageHeight * config.Scale;
                    Canvas.SetLeft(Crosshair, (240 - Crosshair.Width) / 2 + config.OffsetX); Canvas.SetTop(Crosshair, (240 - Crosshair.Height) / 2 + config.OffsetY); Crosshair.Visibility = Visibility.Visible;
                }
            }
            catch { Crosshair.Visibility = Visibility.Collapsed; }
        }

        static bool TryParseHex(string value, out byte red, out byte green, out byte blue)
        {
            red = green = blue = 0; value = (value ?? "").Trim().TrimStart('#');
            return value.Length == 6 && byte.TryParse(value.Substring(0, 2), System.Globalization.NumberStyles.HexNumber, null, out red) && byte.TryParse(value.Substring(2, 2), System.Globalization.NumberStyles.HexNumber, null, out green) && byte.TryParse(value.Substring(4, 2), System.Globalization.NumberStyles.HexNumber, null, out blue);
        }
        static bool ValidHex(string value) => TryParseHex(value, out _, out _, out _);
        static byte ParseHex(string value, int start) => Convert.ToByte(value.Trim().TrimStart('#').Substring(start - 1, 2), 16);
    }
}
