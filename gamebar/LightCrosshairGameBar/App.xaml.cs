using Microsoft.Gaming.XboxGameBar;
using Windows.ApplicationModel.Activation;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;

namespace LightCrosshairGameBar
{
    sealed partial class App : Application
    {
        public App() => InitializeComponent();

        protected override void OnActivated(IActivatedEventArgs args)
        {
            if (!(args is XboxGameBarWidgetActivatedEventArgs widgetArgs) || !widgetArgs.IsLaunchActivation) return;
            var frame = new Frame();
            Window.Current.Content = frame;
            var widget = new XboxGameBarWidget(widgetArgs, Window.Current.CoreWindow, frame);
            if (widgetArgs.AppExtensionId == "Crosshair") frame.Navigate(typeof(CrosshairPage), widget);
            else if (widgetArgs.AppExtensionId == "Settings") frame.Navigate(typeof(SettingsPage), widget);
            Window.Current.Activate();
        }
    }
}
