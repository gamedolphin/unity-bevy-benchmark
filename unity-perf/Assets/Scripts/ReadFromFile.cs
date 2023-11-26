using Unity.Entities;
using System.IO;
using UnityEngine;

public struct FileSpawnerInfo : IComponentData
{
    public bool Exists;
    public int count { get; set; }
    public float maxSize { get; set; }
    public float robotSpeed { get; set; }
}

public partial class ECSSystem : SystemBase
{
    protected override void OnCreate()
    {
        var path = Application.streamingAssetsPath + "/configuration.json";

        string[] args = System.Environment.GetCommandLineArgs();
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i].Contains("-configPath"))
            {
                path = Application.streamingAssetsPath + args[i + 1];
            }
        }

        if (!File.Exists(path))
        {
            var missing = EntityManager.CreateEntity();
            EntityManager.AddComponentData(missing, new FileSpawnerInfo
            {
                Exists = false,
            });

            return;
        }

        var text = File.ReadAllText(path);
        var data = JsonUtility.FromJson<ConfigurationData>(text);

        var entity = EntityManager.CreateEntity();
        EntityManager.AddComponentData(entity, new FileSpawnerInfo
        {
            Exists = true,
            count = data.count,
            maxSize = data.maxSize,
            robotSpeed = data.robotSpeed,
        });

        GameObject.FindGameObjectWithTag("MainCamera").transform.position = new Vector3(data.cameraPosition.x, data.cameraPosition.y, data.cameraPosition.z);

        Enabled = false;
    }

    protected override void OnUpdate()
    {

    }
}

[System.Serializable]
public class ConfigurationData
{
    public int count;
    public float maxSize;
    public float robotSpeed;
    public Vector3 cameraPosition;
}
