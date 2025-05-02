using System;
using System.ComponentModel;
using System.Reflection;
using System.Windows;
using System.Windows.Interop;
using System.Runtime.InteropServices;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;

namespace Ozi.Utilities;

public partial class FmMainWindow : Window
{
    private const int FoldedHeight = 29;
    private const int UnfoldedHeight = 62;
    public bool IsAutoFold;
    private bool _isFolded;
    public readonly FmSlider fmSlider;
    private readonly FmRulers fmRulers;
    private DateTime _localTime;
    private DispatcherTimer? _timeTimer;
    private bool isMouseOver = false;

    // P/Invoke declaration for SetWindowPos - required for making app always on top
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy,
        uint uFlags);

    // Constructor
    public FmMainWindow()
    {
        InitializeComponent();
        // CommonTimeZones.ShowTimeZones();
        fmSlider = new FmSlider(this);
        fmRulers = new FmRulers(this);

        // Subscribe to changes in the App.Settings.Opacity
        App.Settings.PropertyChanged += (s, e) =>
        {
            if (e.PropertyName == nameof(App.Settings.Opacity) && !isMouseOver)
            {
                this.Opacity = App.Settings.Opacity;
            }
        };

        // Initial opacity from settings
        this.Opacity = App.Settings.Opacity;
    }

    private void fmMain_LocationChanged(object sender, EventArgs e)
    {
        fmRulers.Left = Left;
        fmRulers.Top = Top + Height;
        fmSlider.Left = Left;
        fmSlider.Top = Top + Height + fmRulers.Height;
    }

    private void fmMain_Loaded(object sender, RoutedEventArgs e)
    {
        init_Timer();
        read_Config();

        foreach (var timeZoneInfo in App.Settings.TimeZones)
        {
            var clock = new OsClock(timeZoneInfo.Value.Label ?? timeZoneInfo.Key,
                timeZoneInfo.Value.TimeZone,
                timeZoneInfo.Value.Color,
                App.Settings.LstClock.Count,
                timeZoneInfo.Value.IsMain ?? false);
            App.Settings.LstClock.Add(clock);

            gdMain.Children.Add(clock.OsGrid);
            if (timeZoneInfo.Value.IsMain ?? false)
            {
                App.Settings.MainClockIndex = gdMain.Children.Count - 1;
                App.Settings.MainTimeZone = timeZoneInfo.Value.TimeZone;
            }
        }

        FmMain.Width = App.Settings.LstClock.Count * 100 + 1;

        CreateContextMenu();
    }

    private void CreateContextMenu()
    {
        var mainMenu = new ContextMenu();

        var itemClock = new MenuItem { Header = "Clock" };
        mainMenu.Items.Add(itemClock);

        var itemRename = new MenuItem { Header = "Rename" };
        itemRename.Click += MenuItemRename_Click;
        itemClock.Items.Add(itemRename);

        var itemChangeTimeZone = new MenuItem { Header = "Change Time Zone" };
        itemChangeTimeZone.Click += MenuItemChangeTimeZone_Click;
        itemClock.Items.Add(itemChangeTimeZone);

        var itemRemove = new MenuItem { Header = "Remove" };
        itemRemove.Click += MenuItemRemove_Click;
        itemClock.Items.Add(itemRemove);


        var itemAbout = new MenuItem { Header = "About" };
        itemAbout.Click += MenuItemAbout_Click;
        mainMenu.Items.Add(itemAbout);

        var itemSettings = new MenuItem { Header = "Settings" };
        itemSettings.Click += MenuItemSettings_Click;
        mainMenu.Items.Add(itemSettings);

        mainMenu.Items.Add(new Separator());

        var itemExit = new MenuItem { Header = "Exit" };
        itemExit.Click += MenuItemExit_Click;
        mainMenu.Items.Add(itemExit);

        FmMain.ContextMenu = mainMenu;
    }

    private void MenuItemChangeTimeZone_Click(object sender, RoutedEventArgs e)
    {
        throw new NotImplementedException();
    }

    private void MenuItemRename_Click(object sender, RoutedEventArgs e)
    {
        throw new NotImplementedException();
    }


    private void MenuItemRemove_Click(object sender, RoutedEventArgs e)
    {
        fmSlider.Size -= 1;
    }

    private void read_Config()
    {
        Left = App.Settings.MainWndLeft;
        Top = App.Settings.MainWndTop;
        IsAutoFold = App.Settings.IsAutoFold;
    }

    private void init_Timer()
    {
        _timeTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromSeconds(1)
        };
        _timeTimer.Tick += TtTick;
        _timeTimer.Start();
    }


    private void TtTick(object sender, EventArgs e)
    {
        var utcNow = DateTime.UtcNow;
        if (fmSlider.Visibility == Visibility.Visible)
        {
            var localOffset = TimeZoneInfo.Local.GetUtcOffset(utcNow);
            var targetOffset = TimeZoneInfo.FindSystemTimeZoneById(App.Settings.MainTimeZone).GetUtcOffset(utcNow);

            _localTime = fmSlider.CurTime.Date + (localOffset - targetOffset);

            _localTime = _localTime.AddMinutes((int)fmSlider.slTimeChecker.Value * 5);
            fmRulers.rwTop.Height = new GridLength(fmRulers.rwTop.MaxHeight *
                fmSlider.slTimeChecker.Value / fmSlider.slTimeChecker.Maximum);
        }
        else
        {
            _localTime = utcNow;
        }

        foreach (var item in App.Settings.LstClock!)
            item.SetTime(_localTime);
    }

    private void MenuItemSettings_Click(object sender, RoutedEventArgs e)
    {
        var newFmSettings = new FmSettings(this);
        newFmSettings.ShowDialog();
    }

    private void AdjustTimeCheckerPosition()
    {
        fmRulers.Left = Left;
        fmRulers.Top = Top + Height;
        fmSlider.Left = Left;
        fmSlider.Top = Top + Height + fmRulers.Height;
    }

    private void MenuItemTimeChecker_Click(object sender, RoutedEventArgs e)
    {
        if (fmSlider.Visibility == Visibility.Visible)
        {
            fmSlider.Hide();
            fmRulers.Hide();
            _timeTimer!.Interval = TimeSpan.FromSeconds(1);
        }
        else
        {
            var _curTime = TimeZoneInfo.ConvertTimeBySystemTimeZoneId(_localTime, App.Settings.MainTimeZone);
            fmSlider.CurTime = _curTime;
            fmSlider.slTimeChecker.Value = _curTime.Hour * 12 + (int)(_curTime.Minute / 5);
            fmSlider.Show();
            fmRulers.Show();
            _timeTimer!.Interval = TimeSpan.FromMicroseconds(100);
        }

        fmMain_MouseLeave(null!, null!);
    }

    private void MenuItemExit_Click(object sender, RoutedEventArgs e)
    {
        Application.Current.Shutdown();
    }

    private void MenuItemAbout_Click(object sender, RoutedEventArgs e)
    {
        var version = Assembly.GetExecutingAssembly().GetName().Version?.ToString();
        MessageBox.Show("Ozi clock. " + version);
    }


    private void fmMain_MouseDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true; //double click  
        if (e.ClickCount > 1)
        {
            MenuItemTimeChecker_Click(null!, null!);
        }

        // fmSlider.Size = fmSlider.Size - 1;

        if (e.ChangedButton == MouseButton.Left)
        {
            FmMain.DragMove();
        }

        if (e.ChangedButton == MouseButton.Middle)
        {
            if (_isFolded)
            {
                UnFoldMainWindow();
            }
            else
            {
                FoldMainWindow();
            }
        }
    }

    private void FoldingMainWindow()
    {
        var i = (int)Height;
        if (IsMouseOver)
        {
            while (i <= UnfoldedHeight)
                if (IsMouseOver)
                {
                    Height = i++;
                    AdjustTimeCheckerPosition();
                }
                else
                {
                    return;
                }
        }
        else
        {
            while (i >= FoldedHeight)
                if (!IsMouseOver)
                {
                    Height = i--;
                    AdjustTimeCheckerPosition();
                }
                else
                {
                    return;
                }
        }
    }

    private void fmMain_MouseEnter(object sender, MouseEventArgs e)
    {
        isMouseOver = true;
        Opacity = 1;
    }

    private void fmMain_MouseLeave(object sender, MouseEventArgs e)
    {
        isMouseOver = false;
        Opacity = App.Settings.Opacity;
    }

    private void FoldMainWindow()
    {
        {
            for (var i = (int)Height; i > FoldedHeight; i--)
            {
                FmMain.Height = i;
                AdjustTimeCheckerPosition();
            }

            _isFolded = true;
        }
    }

    private void UnFoldMainWindow()
    {
        {
            for (var i = (int)Height; i < UnfoldedHeight; i++)
            {
                FmMain.Height = i;
                AdjustTimeCheckerPosition();
            }

            _isFolded = false;
        }
    }
}


// _lstClock.Add(new OsClock("NYK", "Eastern Standard Time", "#FFAAAAFF", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("LDN", "GMT Standard Time", "#FFAAFFAA", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("KYIV", "FLE Standard Time", "#FFAAFFFF", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("PUN", "India Standard Time", "#FF99BBBB", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("SGP", "Singapore Standard Time", "#FFFFFFAA", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("TKO", "Tokyo Standard Time", "#FFFFAAAA", _lstClock.Count * 100));