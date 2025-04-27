using System.Windows;

namespace Ozi.Utilities;

/// <summary>
/// Interaction logic for App.xaml
/// </summary>
public partial class App
{
    public static AppSettings Settings { get; } = new();

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        Settings.Load();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        base.OnExit(e);
        Settings.Save();
    }
}