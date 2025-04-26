using System;
using System.Collections.Generic;
using System.Reflection;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;


namespace Ozi.Clock;

/// <summary>
/// Interaction logic for MainWindow.xaml
/// </summary>
public partial class MainWindow : Window
{
    private List<OsClock>? _lstClock;
    private double _fTransparentValue = 90;
    private bool _fIsTransparent;
    public bool IsAutoFold;
    private bool _isFolded;
    public readonly FmTimeChecker NewFmTimeChecker;
    private readonly TimeChecker2 _timeChecker2;
    private DateTime _localTime;
    private DispatcherTimer?  _timeTimer;
    public bool UseSnap = true;
    // private Screen[] screens;

    public MainWindow()
    {
        InitializeComponent();

        NewFmTimeChecker = new FmTimeChecker(this);
        _timeChecker2 = new TimeChecker2(this);
        // screens = Screen.AllScreens;
    }

    public bool IsTransparent
    {
        set
        {
            _fIsTransparent = value;
            if (!_fIsTransparent)
                Opacity = 1;
        }
        get { return _fIsTransparent; }
    }

    public double TransparentValue
    {
        set
        {
            _fTransparentValue = value;
            if (_fIsTransparent)
                fmMain.Opacity = _fTransparentValue;
            else
                fmMain.Opacity = 1;
        }
        get { return _fTransparentValue; }
    }

    private void fmMain_Loaded(object sender, RoutedEventArgs e)
    {
        init_Timer();
        read_Config();

        _lstClock = new List<OsClock>();

        _lstClock.Add((new OsClock("NYK", "Eastern Standard Time", "#FFAAAAFF", _lstClock.Count * 100)));
        _lstClock.Add((new OsClock("LDN", "GMT Standard Time", "#FFAAFFAA", _lstClock.Count * 100)));
        _lstClock.Add((new OsClock("KYIV", "FLE Standard Time", "#FFAAFFFF", _lstClock.Count * 100)));
        _lstClock.Add((new OsClock("PUN", "India Standard Time", "#FF99BBBB", _lstClock.Count * 100)));
        _lstClock.Add((new OsClock("SGP", "Singapore Standard Time", "#FFFFFFAA", _lstClock.Count * 100)));
        _lstClock.Add((new OsClock("TKO", "Tokyo Standard Time", "#FFFFAAAA", _lstClock.Count * 100)));

        foreach (var item in _lstClock)
        {
            gdMain.Children.Add(item.OsGrid);
        }

        fmMain.Width = _lstClock.Count * 100 + 1;
        _lstClock[2].IsMain = true;


        //Context menu
        var mainMenu = new ContextMenu();

        var item1 = new MenuItem
        {
            Header = "About"
        };
        //item1.Icon = new System.Windows.Controls.Image
        //{
        //    Source = new BitmapImage(new Uri("Resources/Copy16.png", UriKind.Relative))
        //};
        //new BitmapImage(new Uri("Resources/pencil_16.BMP", UriKind.Relative));
        item1.Click += MenuItemAbout_Click;
        mainMenu.Items.Add(item1);

        var item3 = new MenuItem
        {
            Header = "Settings"
        };
        item3.Click += MenuItemSettings_Click;
        mainMenu.Items.Add(item3);

        //System.Windows.Controls.MenuItem item4 = new System.Windows.Controls.MenuItem();
        //item4.Header = "TimeChecker"; item4.Click += MenuItemTimeChecker_Click; mainMenu.Items.Add(item4);

        mainMenu.Items.Add(new Separator());

        var item5 = new MenuItem
        {
            Header = "Exit"
        };
        item5.Click += MenuItemExit_Click;
        mainMenu.Items.Add(item5);

        fmMain.ContextMenu = mainMenu;
    }

    private void read_Config()
    {
        Left = App.Settings.MainWndLeft;
        Top = App.Settings.MainWndTop;
        IsTransparent = App.Settings.IsTransparent;
        TransparentValue = App.Settings.TransparentValue;
        ShowInTaskbar = App.Settings.ShowInTaskBar;
        Topmost = App.Settings.TopMost;
        IsAutoFold = App.Settings.IsAutoFold;
        UseSnap = App.Settings.UseSnap;
    }

    private void save_Config()
    {
        App.Settings.MainWndLeft = Left;
        App.Settings.MainWndTop = Top;
        App.Settings.IsTransparent = IsTransparent;
        App.Settings.TransparentValue = TransparentValue;
        App.Settings.ShowInTaskBar = ShowInTaskbar;
        App.Settings.TopMost = Topmost;
        App.Settings.IsAutoFold = IsAutoFold;
        App.Settings.UseSnap = UseSnap;

        // App.Settings.Save();
    }

    private void init_Timer()
    {
        _timeTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromSeconds(1)
        };
        _timeTimer.Tick += TtTick!;
        _timeTimer.Start();
    }

    private void TtTick(object sender, EventArgs e)
    {
        if (NewFmTimeChecker.Visibility == Visibility.Visible)
        {
            _localTime = NewFmTimeChecker.CurTime.Date;
            _localTime = _localTime.AddMinutes(((int)NewFmTimeChecker.slTimeChecker.Value) * 5);
            _timeChecker2.rwTop.Height = new GridLength(_timeChecker2.rwTop.MaxHeight *
                NewFmTimeChecker.slTimeChecker.Value / NewFmTimeChecker.slTimeChecker.Maximum);
        }
        else
            _localTime = DateTime.Now;

        foreach (var item in _lstClock!)
            item.SetTime(_localTime);
    }

    private void MenuItemSettings_Click(object sender, RoutedEventArgs e)
    {
        var newFmSettings = new FmSettings(this);
        newFmSettings.ShowDialog();
    }

    private void AdjustTimeCheckerPosition()
    {
        _timeChecker2.Left = Left;
        _timeChecker2.Top = Top + Height;
        NewFmTimeChecker.Left = Left;
        NewFmTimeChecker.Top = Top + Height + _timeChecker2.Height;
    }

    private void MenuItemTimeChecker_Click(object sender, RoutedEventArgs e)
    {
        if (NewFmTimeChecker.Visibility == Visibility.Visible)
        {
            NewFmTimeChecker.Hide();
            _timeChecker2.Hide();
            _timeTimer!.Interval = TimeSpan.FromSeconds(1);
        }
        else
        {
            NewFmTimeChecker.CurTime = _localTime;
            NewFmTimeChecker.slTimeChecker.Value = _localTime.Hour * 12 + (int)(_localTime.Minute / 5);
            NewFmTimeChecker.Show();
            _timeChecker2.Show();
            _timeTimer.Interval = TimeSpan.FromMicroseconds(100);
        }

        fmMain_MouseLeave(null, null);
    }

    private void MenuItemExit_Click(object sender, RoutedEventArgs e)
    {
        Application.Current.Shutdown();
    }

    private void MenuItemAbout_Click(object sender, RoutedEventArgs e)
    {
        string version = null;
        var assem = typeof(MainWindow).Assembly;
        version = Assembly.GetExecutingAssembly().GetName().Version.ToString();

        MessageBox.Show("Ozi clock. " + version);
    }

    private void fmMain_Closing(object sender, System.ComponentModel.CancelEventArgs e)
    {
        save_Config();
    }

    private void gdMain_MouseDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true; //double click  

        if (e.ClickCount > 1) MenuItemTimeChecker_Click(null, null);

        if (e.ChangedButton == MouseButton.Left)
        {
            fmMain.DragMove();
        }

        if (e.ChangedButton == MouseButton.Middle)
            if (_isFolded)
                UnFoldMainWindow();
            else
                FoldMainWindow();
    }

    private void FoldingMainWindow()
    {
        var i = (int)Height;
        if (IsMouseOver)
        {
            while (i <= 62)
                if (IsMouseOver)
                {
                    Height = i++;
                    AdjustTimeCheckerPosition();
                }
                else
                    return;
        }
        else
            while (i >= 29)
                if (!IsMouseOver)
                {
                    Height = i--;
                    AdjustTimeCheckerPosition();
                }
                else
                    return;
    }

    private void fmMain_MouseEnter(object sender, MouseEventArgs e)
    {
        if (IsTransparent)
            Opacity = 1;
        if (IsAutoFold && _isFolded)
            FoldingMainWindow();
    }

    private void fmMain_MouseLeave(object sender, MouseEventArgs e)
    {
        if (IsTransparent && !(NewFmTimeChecker.Visibility == Visibility.Visible))
            Opacity = TransparentValue;
        else
            Opacity = 100;
        if (IsAutoFold && _isFolded)
            FoldingMainWindow();
    }

    private void FoldMainWindow()
    {
        {
            for (var i = (int)Height; i > 29; i--)
            {
                fmMain.Height = i;
                AdjustTimeCheckerPosition();
            }

            _isFolded = true;
        }
    }

    private void UnFoldMainWindow()
    {
        {
            for (var i = (int)Height; i < 62; i++)
            {
                fmMain.Height = i;
                AdjustTimeCheckerPosition();
            }

            _isFolded = false;
        }
    }

    private void fmMain_LocationChanged(object sender, EventArgs e)
    {
        if (UseSnap)
            SnapWindow();
        AdjustTimeCheckerPosition();
    }

    private bool _isAlreadySnapped = false;

    private void SnapWindow()
    {
        if (!UseSnap) return;

        // Screen screenTmp = null;
        // foreach (Screen screen in screens)
        //     if (screen.WorkingArea.Contains((int)(this.Left + this.Width / 2), (int)(this.Top + this.Height / 2)))
        //         screenTmp = screen;

        // if (screenTmp == null) return;
        //
        // if (Math.Abs(screenTmp.WorkingArea.Left - this.Left) <= 20)
        // {
        //     this.Left = screenTmp.WorkingArea.Left;
        // }
        //
        // int borderLim = 0;
        // if (!isAlreadySnapped)
        // {
        //     borderLim = 20;
        // }
        // else
        // {
        //     borderLim = 45;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Top - this.Top) <= borderLim && !isAlreadySnapped)
        // {
        //     this.Top = screenTmp.WorkingArea.Top;
        //     isAlreadySnapped = true;
        // }
        // else
        // {
        //     isAlreadySnapped = false;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Right - (this.Left + this.Width)) <= 20)
        // {
        //     this.Left = screenTmp.WorkingArea.Right - this.Width;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Bottom - (this.Top + this.Height)) <= 20)
        // {
        //     this.Top = screenTmp.WorkingArea.Bottom - this.Height;
        // }
    }
}