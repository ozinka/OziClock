using System.Windows;

namespace Ozi.Utilities;

public partial class App
{
    public static AppSettings Settings { get; } = new();

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        Resources["AppSettings"] = Settings;
        Settings.Load();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        base.OnExit(e);
        Settings.Save();
    }
}